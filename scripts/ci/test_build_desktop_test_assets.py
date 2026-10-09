"""Test-mode asset staging must meet the SDK input contract and clean up."""
import argparse
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec=importlib.util.spec_from_file_location('build_desktop',Path(__file__).resolve().parents[1]/'build-desktop.py')
build=importlib.util.module_from_spec(spec)
spec.loader.exec_module(build)


class SDKReached(Exception):
    pass


class TestModeAssets(unittest.TestCase):
    def test_sdk_receives_index_and_assets_are_cleaned_on_failure(self):
        with tempfile.TemporaryDirectory() as root:
            host=Path(root)
            args=argparse.Namespace(host_dir=root,vivy_dir=root,laputa_dir=root)
            observed=[]
            def require_sdk_assets(args,lock,assets,output):
                asset_root=host/assets
                self.assertTrue((asset_root/'index.html').is_file(),
                    'SDK requires index.html even for backend-only tests')
                self.assertIn('test', (asset_root/'index.html').read_text())
                observed.append(asset_root)
                raise SDKReached()
            with patch.object(build,'stage_tracked'),patch.object(build,'build_lock',return_value={}),patch.object(build,'sdk_pack',side_effect=require_sdk_assets):
                with self.assertRaises(SDKReached):
                    build.mode_test(args)
            self.assertTrue(observed)
            self.assertFalse(observed[0].exists(),'temporary assets must be removed after failed pack')


if __name__=='__main__':
    unittest.main()
