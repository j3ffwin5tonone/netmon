"""Regression checks for stale/missing legal materials; never touches real files."""
import copy
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("licenses", Path(__file__).with_name("licenses.py"))
licenses = importlib.util.module_from_spec(spec)
spec.loader.exec_module(licenses)
SOURCE = licenses.ROOT


class LicenseGuardTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        for name in ("licenses", "static/legal-sources"):
            shutil.copytree(SOURCE / name, self.root / name)
        for name in ("LICENSE", "package.json", "package-lock.json", "src-tauri/Cargo.toml", "src-tauri/Cargo.lock", "src-tauri/tauri.conf.json"):
            dest = self.root / name
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(SOURCE / name, dest)
        licenses.ROOT = self.root
        self.inventory = self.root / "licenses/inventory.json"

    def tearDown(self):
        licenses.ROOT = SOURCE
        self.temp.cleanup()

    def test_current_inventory_and_html_escape(self):
        inventory = licenses.validated_inventory()
        altered = copy.deepcopy(inventory)
        altered["packages"][0]["name"] = '<script>alert("test")</script>'
        page = licenses.render(altered)
        self.assertNotIn('<script>', page)
        self.assertIn('&lt;script&gt;', page)
        self.assertIn('MPL source availability', page)
        self.assertIn('Microsoft.Web.WebView2', page)

    def test_changed_lockfile_requires_review(self):
        with (self.root / "package-lock.json").open("a") as f:
            f.write("\n")
        with self.assertRaisesRegex(ValueError, "changed"):
            licenses.validated_inventory()

    def test_missing_dependency_is_rejected(self):
        data = json.loads(self.inventory.read_text())
        data["packages"].pop()
        self.inventory.write_text(json.dumps(data))
        with self.assertRaisesRegex(ValueError, "exact locked dependency set"):
            licenses.validated_inventory()

    def test_tampered_notice_is_rejected(self):
        data = json.loads(self.inventory.read_text())
        document = data["packages"][0]["documents"][0]
        (self.root / "licenses" / document["file"]).write_text("changed notice")
        with self.assertRaisesRegex(ValueError, "License text changed"):
            licenses.validated_inventory()

    def test_wrong_mpl_source_archive_is_rejected(self):
        data = json.loads(self.inventory.read_text())
        archive = next(p["source_archive"] for p in data["packages"] if "source_archive" in p)
        (self.root / archive["file"]).write_bytes(b"wrong source")
        with self.assertRaisesRegex(ValueError, "MPL source archive"):
            licenses.validated_inventory()


if __name__ == "__main__":
    unittest.main()
