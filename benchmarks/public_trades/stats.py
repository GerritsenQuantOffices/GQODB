"""Statistics and decision rules of docs/B17_PUBLIC_TRADES_PROTOCOL.md (sections 9 and 10)."""
import functools
import math
import random

SEED = 20261017
RESAMPLES = 10_000
BLOCK = 3
BANDS = {"bytes": 0.02, "time": 0.05, "rss": 0.10, "cpu": 0.05}


def median(values):
    ordered = sorted(values)
    n = len(ordered)
    if n == 0:
        raise ValueError("median of nothing")
    mid = n // 2
    return ordered[mid] if n % 2 else (ordered[mid - 1] + ordered[mid]) / 2


def mad(values):
    centre = median(values)
    return median([abs(v - centre) for v in values])


def describe(values):
    return {"n": len(values), "median": median(values), "mad": mad(values),
            "min": min(values), "max": max(values)}


def _fast_median(values):
    ordered = sorted(values)
    n = len(ordered)
    mid = n // 2
    return ordered[mid] if n % 2 else (ordered[mid - 1] + ordered[mid]) / 2


def _ratio_of_medians(a, b, index):
    denominator = _fast_median(map(b.__getitem__, index))
    if denominator == 0:
        return math.inf
    return _fast_median(map(a.__getitem__, index)) / denominator


@functools.lru_cache(maxsize=16)
def _resample_indices(n, length, resamples, seed):
    """The index sets depend only on n, block length, count and seed, so every comparison
    of the same length reuses the same draws (this is also what makes results reproducible)."""
    rng = random.Random(seed)
    draws = []
    for _ in range(resamples):
        index = []
        while len(index) < n:
            start = rng.randrange(0, n - length + 1)
            index.extend(range(start, start + length))
        draws.append(tuple(index[:n]))
    return tuple(draws)


def bootstrap_ratio(a, b, block=BLOCK, resamples=RESAMPLES, seed=SEED, iid=False):
    """95% percentile interval of median(a)/median(b), resampling rounds in pairs.

    a and b are per-round values in chronological order. With iid=False a moving-block
    bootstrap of the given block length is used (protocol: block length 3); with iid=True
    single rounds are drawn (the sensitivity result)."""
    n = len(a)
    if n != len(b) or n < 2:
        raise ValueError("need two paired series of at least two rounds")
    length = 1 if iid else min(block, n)
    draws = sorted(_ratio_of_medians(a, b, index) for index in _resample_indices(n, length, resamples, seed))
    lo = draws[int(0.025 * resamples)]
    hi = draws[min(resamples - 1, int(math.ceil(0.975 * resamples)) - 1)]
    return lo, hi


def decide(rho, lo, hi, band):
    """BETTER / WORSE / EQUIVALENT / INCONCLUSIVE for a ratio gqodb / baseline (smaller is
    better). Rules are tested in this order, which only matters for a ratio that sits exactly
    on a band edge."""
    if hi < 1 and rho <= 1 - band:
        return "BETTER"
    if lo > 1 and rho >= 1 + band:
        return "WORSE"
    if lo >= 1 - band and hi <= 1 + band:
        return "EQUIVALENT"
    return "INCONCLUSIVE"


def compare(gqodb, baseline, kind, deterministic=False):
    """Ratio summary and verdict for two per-round series (gqodb / baseline)."""
    band = BANDS[kind]
    rho = _ratio_of_medians(gqodb, baseline, range(len(gqodb)))
    if deterministic:
        lo = hi = rho
        iid = (rho, rho)
    else:
        lo, hi = bootstrap_ratio(gqodb, baseline)
        iid = bootstrap_ratio(gqodb, baseline, iid=True)
    return {"ratio": rho, "lo": lo, "hi": hi, "iid_lo": iid[0], "iid_hi": iid[1],
            "n": len(gqodb), "band": band, "verdict": decide(rho, lo, hi, band)}
