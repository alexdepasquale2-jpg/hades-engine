from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
RULESETS_DIR = REPO_ROOT / "rulesets"
MECHANICS_DIR = REPO_ROOT / "mechanics"
EXTENSIONS_DIR = MECHANICS_DIR / "extensions"
SCHEMA_DIR = MECHANICS_DIR / "schema"
CATALOG_DIR = MECHANICS_DIR / "catalog"
