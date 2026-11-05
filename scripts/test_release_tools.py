"""Synthetic package checks; never inspect or modify production profile data."""
import json
from pathlib import Path
import tempfile
import unittest
import release_tools as release

class PackageTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory(prefix="release-test-",dir=release.ROOT/"target")
        self.root=Path(self.temp.name);self.directory=self.root/"dist/AttentionFocus"
        self.directory.mkdir(parents=True);(self.root/"target").mkdir();(self.root/"docs").mkdir()
        (self.root/"Cargo.toml").write_text('[package]\nname="synthetic"\nversion="1.2.3"\n',encoding="utf-8")
        (self.root/"docs/user-guide.md").write_text("Synthetic guide source",encoding="utf-8")
        for name,text in [("attention-focus.exe","Fake EXE, never run"),("User-Guide-1.2.3.pdf","Fake PDF"),("focus.sqlite3","Private history, must not ship")]:
            (self.directory/name).write_text(text,encoding="utf-8")
        release.stamp(self.root)
    def tearDown(self):self.temp.cleanup()
    def released(self,folder):return json.loads((folder/"release.json").read_bytes())
    def test_exact_allowlist_and_current_hashes_exclude_private_history(self):
        folder=release.pack(self.root);release.verify(self.root)
        self.assertEqual({p.name for p in folder.iterdir()},{"attention-focus.exe","User-Guide-1.2.3.pdf","release.json"})
        (self.directory/"attention-focus.exe").write_text("Different fake bytes",encoding="utf-8")
        with self.assertRaises(ValueError):release.verify(self.root)
        with self.assertRaises(ValueError):release.pack(self.root)
        release.stamp(self.root);release.pack(self.root);release.verify(self.root)
        self.assertEqual((folder/"attention-focus.exe").read_text(encoding="utf-8"),"Different fake bytes")
    def test_a_changed_or_extra_published_file_fails_verification(self):
        folder=release.pack(self.root)
        (folder/"User-Guide-1.2.3.pdf").write_text("Tampered",encoding="utf-8")
        with self.assertRaises(ValueError):release.verify(self.root)
        release.pack(self.root);release.verify(self.root)
        (folder/"extra.txt").write_text("Not part of a release",encoding="utf-8")
        with self.assertRaises(ValueError):release.verify(self.root)
    def test_packing_refuses_to_remove_unknown_files(self):
        folder=release.pack(self.root)
        (folder/"notes.txt").write_text("Somebody's file",encoding="utf-8")
        with self.assertRaises(ValueError):release.pack(self.root)
        self.assertTrue((folder/"notes.txt").exists())
    def test_a_leftover_from_an_interrupted_swap_does_not_block_packing(self):
        folder=release.pack(self.root)
        leftover=folder.with_name(folder.name+".previous");leftover.mkdir()
        (leftover/"attention-focus.exe").write_text("Old fake bytes",encoding="utf-8")
        release.pack(self.root);release.verify(self.root)
        self.assertFalse(leftover.exists())
    def test_source_change_requires_new_version_and_keeps_the_old_release(self):
        folder=release.pack(self.root);original=self.released(folder)
        (self.root/"docs/user-guide.md").write_text("Changed guide source",encoding="utf-8")
        with self.assertRaises(ValueError):release.pack(self.root)
        self.assertEqual(self.released(folder),original)
        (self.root/"Cargo.toml").write_text('[package]\nname="synthetic"\nversion="1.2.4"\n',encoding="utf-8")
        (self.directory/"User-Guide-1.2.4.pdf").write_text("New guide",encoding="utf-8")
        release.stamp(self.root)
        newer=release.pack(self.root);release.verify(self.root)
        self.assertNotEqual(newer,folder)
        self.assertEqual(self.released(folder),original)
    def test_missing_required_file_keeps_the_previous_release(self):
        folder=release.pack(self.root);original=self.released(folder)
        (self.directory/"User-Guide-1.2.3.pdf").unlink()
        with self.assertRaises(ValueError):release.pack(self.root)
        self.assertEqual(self.released(folder),original)
        self.assertTrue((folder/"User-Guide-1.2.3.pdf").exists())
    def test_paths_and_version_are_validated(self):
        with self.assertRaises(ValueError):release.safe_file(self.root.parent/"outside.txt",self.root)
        (self.root/"Cargo.toml").write_text('[package]\nversion="1.2.3-dev"\n',encoding="utf-8")
        with self.assertRaises(ValueError):release.version(self.root)
    def test_stale_build_cannot_be_labeled_as_current(self):
        (self.root/"docs/user-guide.md").write_text("Changed without building",encoding="utf-8")
        with self.assertRaises(ValueError):release.pack(self.root)

if __name__=="__main__":unittest.main()
