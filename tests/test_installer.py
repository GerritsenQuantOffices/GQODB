"""Exercise prerequisite consent without network, sudo or real package installs."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


INSTALLER = Path(__file__).resolve().parents[1] / "install.sh"


class InstallerConsent(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="gqodb-installer-test.")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.trace = self.root / "trace"
        self.trace.touch()
        self.env = dict(os.environ, PATH=str(self.bin), CARGO_HOME=str(self.root / "cargo"),
                        TEST_ROOT=str(self.root), TEST_TRACE=str(self.trace))
        self.env.pop("GQODB_RUST_TOOLCHAIN", None)
        for command in ("id", "uname", "sha256sum", "install", "mktemp", "rm", "sh"):
            (self.bin / command).symlink_to(shutil.which(command))
        for command in ("cc", "make"):
            (self.bin / command).symlink_to(shutil.which("true"))
        self.write("git-stub", 'printf "git\\n" >> "$TEST_TRACE"\nexit 42\n')
        (self.bin / "git").symlink_to(self.root / "git-stub")
        self.write("rustup-stub", '''printf 'rustup %s\\n' "$*" >> "$TEST_TRACE"
if [ "$1" = toolchain ]; then /usr/bin/touch "$TEST_ROOT/ready"; exit 0; fi
if [ -f "$TEST_ROOT/ready" ]; then echo 'rustc 1.96.0 (fixture)'; exit 0; fi
exit 1
''')
        self.write("bootstrap", '''printf 'bootstrap %s\\n' "$*" >> "$TEST_TRACE"
/bin/mkdir -p "$CARGO_HOME/bin"
/bin/cp "$TEST_ROOT/rustup-stub" "$CARGO_HOME/bin/rustup"
/bin/chmod 755 "$CARGO_HOME/bin/rustup"
/usr/bin/touch "$TEST_ROOT/ready"
''')
        self.write("bin/curl", '''printf 'curl\\n' >> "$TEST_TRACE"
while [ "$#" -gt 0 ]; do
    case "$1" in -o) output=$2; shift 2 ;; *) shift ;; esac
done
/bin/cat "$TEST_ROOT/bootstrap" > "$output"
''')
        self.write("bin/sudo", 'printf "sudo\\n" >> "$TEST_TRACE"\nexec "$@"\n')
        self.write("bin/apt-get", '''printf 'apt %s\\n' "$*" >> "$TEST_TRACE"
if [ "$1" = install ]; then /bin/ln -s "$TEST_ROOT/git-stub" "$TEST_ROOT/bin/git"; fi
''')

    def write(self, name, body):
        path = self.root / name
        path.write_text("#!/bin/sh\nset -eu\n" + body)
        path.chmod(0o755)

    def run_installer(self, answers="", *arguments):
        result = subprocess.run(["/bin/sh", str(INSTALLER), "--prefix", str(self.root / "prefix"),
                                 *arguments], input=answers, text=True, capture_output=True,
                                env=self.env, timeout=10)
        return result, self.trace.read_text()

    def existing_rustup(self, ready=False):
        (self.bin / "rustup").symlink_to(self.root / "rustup-stub")
        if ready:
            (self.root / "ready").touch()

    def test_rust_refusal_blank_and_eof_never_download(self):
        for answer in ("n\n", "\n", ""):
            with self.subTest(answer=repr(answer)):
                self.trace.write_text("")
                result, trace = self.run_installer(answer)
                self.assertEqual(result.returncode, 1)
                self.assertIn("Rust installation declined", result.stderr)
                self.assertEqual(trace, "")

    def test_fresh_rust_requires_yes_then_bootstraps_without_sudo(self):
        result, trace = self.run_installer("y\n")
        self.assertEqual(result.returncode, 42, result.stderr)
        self.assertIn("bootstrap -y --no-modify-path --profile minimal --default-toolchain 1.96.0", trace)
        self.assertIn("git\n", trace)
        self.assertNotIn("sudo", trace)
        self.assertNotIn("apt ", trace)

    def test_existing_rustup_toolchain_refusal_and_acceptance(self):
        self.existing_rustup()
        result, trace = self.run_installer("n\n")
        self.assertEqual(result.returncode, 1)
        self.assertNotIn("toolchain install", trace)
        result, trace = self.run_installer("y\n")
        self.assertEqual(result.returncode, 42, result.stderr)
        self.assertIn("toolchain install 1.96.0 --profile minimal", trace)
        self.assertNotIn("curl", trace)

    def test_matching_rustup_needs_no_consent_or_download(self):
        self.existing_rustup(ready=True)
        result, trace = self.run_installer()
        self.assertEqual(result.returncode, 42, result.stderr)
        self.assertNotIn("[y/N]", result.stderr)
        self.assertNotIn("curl", trace)
        self.assertNotIn("toolchain install", trace)

    def test_matching_active_compiler_is_reused(self):
        self.write("bin/rustc", "echo 'rustc 1.96.0 (fixture)'\n")
        (self.bin / "cargo").symlink_to(shutil.which("true"))
        result, trace = self.run_installer()
        self.assertEqual(result.returncode, 42, result.stderr)
        self.assertNotIn("[y/N]", result.stderr)
        self.assertEqual(trace, "git\n")

    def test_system_and_rust_permissions_are_separate(self):
        (self.bin / "git").unlink()
        result, trace = self.run_installer("n\n")
        self.assertEqual(result.returncode, 1)
        self.assertEqual(trace, "")
        result, trace = self.run_installer("y\nn\n")
        self.assertEqual(result.returncode, 1)
        self.assertIn("apt update", trace)
        self.assertIn("apt install -y --no-install-recommends git", trace)
        self.assertNotIn("curl", trace)
        self.assertIn("Rust installation declined", result.stderr)

    def test_force_does_not_approve_prerequisites(self):
        result, trace = self.run_installer("", "--force")
        self.assertEqual(result.returncode, 1)
        self.assertEqual(trace, "")

    def test_existing_binary_is_refused_before_any_prerequisite_action(self):
        destination = self.root / "prefix/bin/gqodb-codec"
        destination.parent.mkdir(parents=True)
        destination.write_text("keep this")
        result, trace = self.run_installer("y\ny\n")
        self.assertEqual(result.returncode, 1)
        self.assertEqual(trace, "")
        self.assertEqual(destination.read_text(), "keep this")


if __name__ == "__main__":
    unittest.main()
