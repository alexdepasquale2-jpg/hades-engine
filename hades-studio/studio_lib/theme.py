STUDIO_CSS = """
<style>
/* Engine-style chrome */
header[data-testid="stHeader"] { background: #0a0c10; border-bottom: 1px solid #2a3344; }
.block-container { padding-top: 0.5rem; padding-bottom: 1.5rem; max-width: 100%; }
section[data-testid="stSidebar"] {
  background: linear-gradient(180deg, #12161f 0%, #0c0e14 100%);
  border-right: 1px solid #2a3548;
  min-width: 260px;
}
.studio-toolbar {
  display: flex; align-items: center; gap: 12px;
  padding: 8px 12px; margin: -0.5rem -1rem 0.75rem -1rem;
  background: #161b26; border-bottom: 1px solid #2a3344;
}
.studio-toolbar-title {
  font-size: 0.95rem; font-weight: 700; color: #e8eef8;
  letter-spacing: 0.04em; text-transform: uppercase;
}
.studio-toolbar-sub { font-size: 0.78rem; color: #8899b0; margin-left: 8px; }
.studio-panel-label {
  font-size: 0.7rem; font-weight: 600; color: #6b7c94;
  text-transform: uppercase; letter-spacing: 0.08em; margin-bottom: 4px;
}
.studio-hierarchy-item {
  padding: 4px 8px; border-radius: 4px; font-size: 0.85rem;
  color: #c5d0e0;
}
div[data-testid="stVerticalBlockBorderWrapper"] {
  border-color: #2a3548 !important;
  background: #12161f;
}
</style>
"""
