"""Shared helpers of the B17 harness and adapters (docs/B17_PUBLIC_TRADES_PROTOCOL.md).

Pure standard library at import time; pyarrow is imported lazily where a table is hashed.
Contains the canonical digest (protocol section 3, second implementation next to the Rust
one), the independent CSV reference (R2), the common fsync procedure, cache eviction and
residency, size accounting and process/CPU counters.
"""
import array
import ctypes
import hashlib
import json
import mmap
import os
import re
import sys
import time

assert sys.byteorder == "little", "digest definition assumes little-endian hosts"

SYMBOL = "PAXGUSDT"
BLOCK_ROWS = 32768
WINDOW_US = 60_000_000
SCALE = 10**8
I64_MAX = 2**63 - 1

COLUMNS = {
    "v1": [("agg_id", "i64"), ("price", "f64"), ("qty", "f64"), ("first_id", "i64"),
           ("last_id", "i64"), ("transact_time", "i64"), ("is_buyer_maker", "bool"),
           ("is_best_match", "bool"), ("symbol", "utf8")],
    "v2": [("agg_id", "i64"), ("price_e8", "i64"), ("qty_e8", "i64"), ("first_id", "i64"),
           ("last_id", "i64"), ("transact_time", "i64"), ("is_buyer_maker", "bool"),
           ("is_best_match", "bool"), ("symbol", "utf8")],
}

# ------------------------------------------------------------------ digest (R2)


def _header(name, token, rows):
    return f"col|{name}|{token}|{rows}\n".encode()


def digest_columns(columns, rows):
    """columns: list of (name, token, data). i64 -> array('q'), f64 -> array('d') (bit
    patterns), bool -> bytes of 0/1, utf8 -> list of bytes. Returns the hex digest."""
    h = hashlib.sha256()
    for name, token, data in columns:
        h.update(_header(name, token, rows))
        if token in ("i64", "f64", "bool"):
            h.update(data.tobytes() if hasattr(data, "tobytes") else bytes(data))
        else:
            h.update(sum(len(s) for s in data).to_bytes(8, "little"))
            h.update(b"".join(len(s).to_bytes(4, "little") for s in data))
            h.update(b"".join(data))
    return h.hexdigest()


_LINE = re.compile(rb"^([0-9]+),([0-9]+\.[0-9]{8}),([0-9]+\.[0-9]{8}),([0-9]+),([0-9]+),([0-9]+),(True|False),(True|False)$")


def csv_reference(lines, variant, symbol=SYMBOL):
    """Independent reference (R2): parse CSV lines (bytes, no newline) with int()/float() and
    return {rows, digest, window:{start,end,rows,digest}}. Applies the same strict checks as
    the Rust converter."""
    cols = {k: [] for k in ("agg", "p", "q", "f", "l", "t", "m", "b")}
    prev_agg = prev_t = None
    for number, line in enumerate(lines, 1):
        m = _LINE.match(line)
        if not m:
            raise ValueError(f"row {number}: not in the registered CSV grammar")
        agg, p, q, f, l, t = (int(m.group(1)), m.group(2), m.group(3),
                              int(m.group(4)), int(m.group(5)), int(m.group(6)))
        if any(v > I64_MAX for v in (agg, f, l, t)):
            raise ValueError(f"row {number}: integer exceeds int64")
        if prev_agg is not None and agg <= prev_agg:
            raise ValueError(f"row {number}: agg_id not strictly increasing")
        if prev_t is not None and t < prev_t:
            raise ValueError(f"row {number}: transact_time decreases")
        prev_agg, prev_t = agg, t
        if variant == "v1":
            cols["p"].append(float(p))
            cols["q"].append(float(q))
        else:
            for key, text in (("p", p), ("q", q)):
                whole, frac = text.decode().split(".")
                value = int(whole) * SCALE + int(frac)
                if value > I64_MAX:
                    raise ValueError(f"row {number}: V2 value overflows int64")
                cols[key].append(value)
        cols["agg"].append(agg)
        cols["f"].append(f)
        cols["l"].append(l)
        cols["t"].append(t)
        cols["m"].append(1 if m.group(7) == b"True" else 0)
        cols["b"].append(1 if m.group(8) == b"True" else 0)
    n = len(cols["agg"])
    if n == 0:
        raise ValueError("no rows")

    def build(select):
        ids = select
        pick = lambda key: [cols[key][i] for i in ids]  # noqa: E731
        pf = "d" if variant == "v1" else "q"
        names = [c[0] for c in COLUMNS[variant]]
        data = [
            array.array("q", pick("agg")), array.array(pf, pick("p")), array.array(pf, pick("q")),
            array.array("q", pick("f")), array.array("q", pick("l")), array.array("q", pick("t")),
            bytes(pick("m")), bytes(pick("b")), [symbol.encode()] * len(ids),
        ]
        tokens = [c[1] for c in COLUMNS[variant]]
        return digest_columns(list(zip(names, tokens, data)), len(ids))

    start = cols["t"][n // 2]
    end = min(start + WINDOW_US, I64_MAX)
    window_ids = [i for i in range(n) if start <= cols["t"][i] <= end]
    return {"rows": n, "digest": build(list(range(n))),
            "window": {"start": start, "end": end, "rows": len(window_ids), "digest": build(window_ids)}}


# ------------------------------------------------------------- digest (Arrow)


def _chunks(column):
    return column.chunks if hasattr(column, "chunks") else [column]


def table_is_sorted_by_agg_id(table):
    import pyarrow.compute as pc
    col = table.column("agg_id").combine_chunks()
    if len(col) < 2:
        return True
    return bool(pc.all(pc.less(col.slice(0, len(col) - 1), col.slice(1))).as_py())


def digest_table(table):
    """Digest of a pyarrow Table in the order given (protocol section 3). Raises on nulls."""
    import pyarrow as pa
    import pyarrow.compute as pc
    h = hashlib.sha256()
    rows = table.num_rows
    for field, column in zip(table.schema, table.columns):
        if column.null_count:
            raise ValueError(f"null in column {field.name}")
        t = field.type
        if pa.types.is_int64(t):
            token = "i64"
        elif pa.types.is_float64(t):
            token = "f64"
        elif pa.types.is_boolean(t):
            token = "bool"
        elif pa.types.is_string(t) or pa.types.is_large_string(t) or pa.types.is_string_view(t):
            token = "utf8"
        else:
            raise ValueError(f"unsupported type {t} for {field.name}")
        h.update(_header(field.name, token, rows))
        if token in ("i64", "f64"):
            for chunk in _chunks(column):
                if len(chunk):
                    h.update(memoryview(chunk.buffers()[1])[chunk.offset * 8:(chunk.offset + len(chunk)) * 8])
        elif token == "bool":
            for chunk in _chunks(column):
                if len(chunk):
                    flags = chunk.cast(pa.uint8())
                    h.update(memoryview(flags.buffers()[1])[flags.offset:flags.offset + len(flags)])
        else:
            chunks = [c.cast(pa.large_string()) for c in _chunks(column) if len(c)]
            total = sum((pc.sum(pc.binary_length(c)).as_py() or 0) for c in chunks)
            h.update(total.to_bytes(8, "little"))
            for c in chunks:
                lengths = pc.binary_length(c).cast(pa.uint32())
                h.update(memoryview(lengths.buffers()[1])[lengths.offset * 4:(lengths.offset + len(lengths)) * 4])
            for c in chunks:
                offsets = memoryview(c.buffers()[1]).cast("q")
                start, stop = offsets[c.offset], offsets[c.offset + len(c)]
                data = c.buffers()[2]
                if data is not None and stop > start:
                    h.update(memoryview(data)[start:stop])
    return h.hexdigest()


def canonical_digest(table):
    """Sorts by agg_id when the tool returned another physical order, then hashes.
    Returns (digest, sorted_by_us)."""
    if table_is_sorted_by_agg_id(table):
        return digest_table(table), False
    return digest_table(table.sort_by("agg_id")), True


# ---------------------------------------------------------- filesystem helpers


def fsync_tree(root):
    """Protocol section 5: every regular file, then every directory bottom-up, then the parent."""
    files, dirs = [], []
    if os.path.isdir(root):
        for base, names, filenames in os.walk(root):
            dirs.append(base)
            files.extend(os.path.join(base, f) for f in filenames if not os.path.islink(os.path.join(base, f)))
    else:
        files.append(root)
    for path in files:
        fd = os.open(path, os.O_RDONLY)
        try:
            os.fsync(fd)
        finally:
            os.close(fd)
    for path in sorted(dirs, key=lambda p: -p.count(os.sep)):
        fd = os.open(path, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(fd)
        finally:
            os.close(fd)
    parent = os.path.dirname(os.path.abspath(root)) or "."
    fd = os.open(parent, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def iter_files(root):
    if os.path.isdir(root):
        for base, _names, filenames in os.walk(root):
            for f in filenames:
                path = os.path.join(base, f)
                if not os.path.islink(path):
                    yield path
    elif os.path.isfile(root):
        yield root


def tree_bytes(root):
    """Apparent bytes, allocated bytes (st_blocks * 512) and file count of everything below root."""
    apparent = allocated = count = 0
    for path in iter_files(root):
        st = os.lstat(path)
        apparent += st.st_size
        allocated += st.st_blocks * 512
        count += 1
    return {"apparent": apparent, "allocated": allocated, "files": count}


def evict(root):
    """posix_fadvise(DONTNEED) on the files below root only. Never a global cache drop."""
    for path in iter_files(root):
        fd = os.open(path, os.O_RDONLY)
        try:
            os.posix_fadvise(fd, 0, 0, os.POSIX_FADV_DONTNEED)
        finally:
            os.close(fd)


_libc = None


def resident_fraction(root):
    """Fraction of the pages of all files below root that are in the page cache (mincore)."""
    global _libc
    if _libc is None:
        _libc = ctypes.CDLL(None, use_errno=True)
        _libc.mincore.argtypes = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_void_p]
    page = mmap.PAGESIZE
    resident = total = 0
    for path in iter_files(root):
        size = os.path.getsize(path)
        if size == 0:
            continue
        with open(path, "rb") as handle:
            mm = mmap.mmap(handle.fileno(), size, access=mmap.ACCESS_COPY)
            try:
                pages = (size + page - 1) // page
                vec = (ctypes.c_ubyte * pages)()
                view = ctypes.c_char.from_buffer(mm)
                try:
                    rc = _libc.mincore(ctypes.addressof(view), size, vec)
                finally:
                    del view
                if rc != 0:
                    raise OSError(ctypes.get_errno(), "mincore failed")
                resident += sum(v & 1 for v in vec)
                total += pages
            finally:
                mm.close()
    return resident / total if total else 0.0


# ------------------------------------------------------------ process counters


def vm_hwm_kb():
    try:
        with open("/proc/self/status") as handle:
            for line in handle:
                if line.startswith("VmHWM:"):
                    return int(line.split()[1])
    except OSError:
        pass
    return 0


def now_ns():
    return time.perf_counter_ns()


def emit(payload):
    """Adapter contract: one JSON object as the last stdout line."""
    sys.stdout.write(json.dumps(payload, separators=(",", ":")) + "\n")
    sys.stdout.flush()


def safe_text(value, allowed=r"[A-Za-z0-9_./:+=,@-]+"):
    """Refuse anything that could break out of a SQL string literal or a shell word."""
    if not re.fullmatch(allowed, value):
        raise ValueError(f"unsafe text: {value!r}")
    return value
