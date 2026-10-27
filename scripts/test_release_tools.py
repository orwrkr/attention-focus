"""Synthetic package checks; never inspect or modify production profile data."""
import json
from pathlib import Path
import tempfile
import unittest
import zipfile
import release_tools as release

class PackageTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory(prefix="release-test-",dir=release.ROOT/"target")
        self.root=Path(self.temp.name);self.directory=self.root/"dist/AttentionFocus"
        self.directory.mkdir(parents=True);(self.root/"target").mkdir();(self.root/"docs").mkdir()
        (self.root/"Cargo.toml").write_text('[package]\nname="synthetic"\nversion="1.2.3"\n',encoding="utf-8")
        (self.root/"docs/user-guide.md").write_text("Synthetic guide source",encoding="utf-8")
        for name,text in [("attention-focus.exe","Fake EXE, never run"),("User-Guide-1.2.3.pdf","Fake PDF"),("THIRD-PARTY-NOTICES.txt","Example notices"),("focus.sqlite3","Private history, must not ship")]:
            (self.directory/name).write_text(text,encoding="utf-8")
        release.stamp(self.root)
    def tearDown(self):self.temp.cleanup()
    def test_exact_allowlist_and_current_hashes_exclude_private_history(self):
        archive=release.pack(self.root);release.verify(self.root)
        with zipfile.ZipFile(archive) as z:
            self.assertEqual(len(z.namelist()),4)
            self.assertFalse(any("sqlite" in name for name in z.namelist()))
        (self.directory/"attention-focus.exe").write_text("Different fake bytes",encoding="utf-8")
        with self.assertRaises(ValueError):release.verify(self.root)
        with self.assertRaises(ValueError):release.pack(self.root)
        release.stamp(self.root);release.pack(self.root);release.verify(self.root)
    def test_source_change_requires_new_version_and_keeps_old_archive(self):
        archive=release.pack(self.root);original=archive.read_bytes()
        (self.root/"docs/user-guide.md").write_text("Changed guide source",encoding="utf-8")
        with self.assertRaises(ValueError):release.pack(self.root)
        self.assertEqual(archive.read_bytes(),original)
        (self.root/"Cargo.toml").write_text('[package]\nname="synthetic"\nversion="1.2.4"\n',encoding="utf-8")
        (self.directory/"User-Guide-1.2.4.pdf").write_text("New guide",encoding="utf-8")
        release.stamp(self.root)
        release.pack(self.root);release.verify(self.root)
        self.assertTrue(archive.exists())
    def test_missing_required_file_does_not_replace_archive(self):
        archive=release.pack(self.root);original=archive.read_bytes()
        (self.directory/"THIRD-PARTY-NOTICES.txt").unlink()
        with self.assertRaises(ValueError):release.pack(self.root)
        self.assertEqual(original,archive.read_bytes())
    def test_paths_and_version_are_validated(self):
        with self.assertRaises(ValueError):release.safe_file(self.root.parent/"outside.zip",self.root)
        (self.root/"Cargo.toml").write_text('[package]\nversion="1.2.3-dev"\n',encoding="utf-8")
        with self.assertRaises(ValueError):release.version(self.root)
    def test_stale_build_cannot_be_labeled_as_current(self):
        (self.root/"docs/user-guide.md").write_text("Changed without building",encoding="utf-8")
        with self.assertRaises(ValueError):release.pack(self.root)

if __name__=="__main__":unittest.main()
