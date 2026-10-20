# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg
"""Developer helper: merge new keys into every locale file.

    python3 scripts/addkeys.py new_keys.json     # {"key": ["fr", "en", "es", "de"], ...}

Files stay sorted; run `npm run check:i18n` afterwards.
"""
import json, sys, pathlib
LANGS = ['fr', 'en', 'es', 'de']
d = pathlib.Path(__file__).resolve().parent.parent / 'src/lib/i18n/locales'
new = json.load(open(sys.argv[1]))
for i, lang in enumerate(LANGS):
    p = d / f'{lang}.json'
    data = json.load(open(p))
    for key, values in new.items():
        data[key] = values[i]
    json.dump(dict(sorted(data.items())), open(p, 'w'), ensure_ascii=False, indent='\t')
    open(p, 'a').write('\n')
print('merged', len(new), 'keys into', ', '.join(LANGS))
