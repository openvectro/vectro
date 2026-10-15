"""Build data/rive-keys.json from rive-runtime's generated headers (MIT, (c) Rive).

Usage: python3 tools/extract_rive_keys.py <path-to-rive-runtime> > data/rive-keys.json
"""
import json, re, sys
from pathlib import Path

root = Path(sys.argv[1])
gen = root / "include/rive/generated"
types, props = {}, {}
for f in sorted(gen.rglob("*_base.hpp")):
    src = f.read_text()
    m = re.search(r"class (\w+)Base\s*:\s*public (\w+)", src)
    if not m:
        continue
    name, parent = m.group(1), m.group(2)
    parent = parent[:-4] if parent.endswith("Base") else parent
    tk = re.search(r"static const uint16_t typeKey = (\d+);", src)
    own = {}
    for pm in re.finditer(r"static const uint16_t (\w+)PropertyKey = (\d+);", src):
        own[int(pm.group(2))] = pm.group(1)
        props[int(pm.group(2))] = {"name": pm.group(1), "owner": name}
    if tk:
        types[int(tk.group(1))] = {"name": name, "parent": parent, "properties": own}

reg = (gen / "core_registry.hpp").read_text()
body = reg[reg.index("static int propertyFieldId(int propertyKey)"):]
body = body[: body.index("\n    }\n", body.index("switch"))]
pending = []
for line in body.splitlines():
    c = re.search(r"case (\w+)Base::(\w+)PropertyKey:", line)
    if c:
        pending.append((c.group(1), c.group(2)))
        continue
    r = re.search(r"return Core(\w+)Type::id;", line)
    if r:
        for owner, pname in pending:
            for k, p in props.items():
                if p["owner"] == owner and p["name"] == pname:
                    p["type"] = r.group(1).lower()
        pending = []

json.dump({"source": "rive-app/rive-runtime include/rive/generated (MIT, (c) Rive)",
           "types": dict(sorted(types.items())), "properties": dict(sorted(props.items()))},
          sys.stdout, indent=1)
