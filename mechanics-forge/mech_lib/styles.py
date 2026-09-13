FORGE_CSS = """
<style>
/* Layout */
.block-container { padding-top: 1rem; padding-bottom: 2rem; max-width: 1400px; }
section[data-testid="stSidebar"] {
  background: linear-gradient(180deg, #141a24 0%, #0d1118 100%);
  border-right: 1px solid #2a3548;
}
/* Status chips */
.forge-chip {
  display: inline-block; padding: 2px 10px; border-radius: 999px;
  font-size: 0.75rem; font-weight: 600; letter-spacing: 0.02em;
}
.forge-chip-dirty { background: #3d2a14; color: #f0c674; border: 1px solid #6b4a1a; }
.forge-chip-clean { background: #1a2e24; color: #8fd4a8; border: 1px solid #2d5a42; }
.forge-chip-err { background: #3a1a1a; color: #f0a0a0; border: 1px solid #6b2a2a; }
/* Section headers */
h3.forge-section { margin-top: 0.25rem; font-size: 1.05rem; color: #c8d4e8; }
</style>
"""
