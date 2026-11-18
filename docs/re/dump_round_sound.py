# dump_round_sound.py — the tables of FIGHT_DATA.md sections 13-15: round start / win / lose motions per moveset,
# announcer ids, sample groups and their bank files, voice tables per body, hit-sound tables, floor sounds and arena
# music. python3 dump_round_sound.py > round_sound.json
import json, struct, sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ttt
V, d = ttt.V, ttt.d
def h(a, n): return list(struct.unpack_from('<%dH' % n, d, a - V))
def sh(a, n): return list(struct.unpack_from('<%dh' % n, d, a - V))
def name(p): return d[p - V:d.index(b'\0', p - V)].decode('latin1')

# round machine 0x36E198 / 0x368398 / 0x368528 / 0x368748
ALIASES = {'intro_a': 4, 'later_round': 5, 'intro_b': 6, 'unknown_intro': 9,
           'win_0': 0x12A2, 'win_1': 0x12A3, 'win_2': 0x12A4, 'win_3': 0x12A5, 'team_lose': 0x12AC}
F = ttt.frames()
motions = {}
for n, r in sorted(ttt.roster().items(), key=lambda x: x[1]['body']):
    ms = r['moveset']
    if ms >= ttt.NSETS: continue
    row = {}
    for k, a in ALIASES.items():
        m = ttt.alias(ms, a)
        row[k] = None if m == 7350 else {'move': m, 'frames': F[m]}
    motions[n] = {'body': r['body'], 'moveset': ms, **row}

# sample groups (0x42EBD8, 62 x 16 bytes): entries ptr, bank bytes, kind, first id, count
FILES = [1] + [187 + 7 * k for k in range(20)] + ['327 or 334', 45]
groups = []
for i in range(62):
    p, size, kind, first, count = struct.unpack_from('<IIHHH', d, 0x42EBD8 - V + 16 * i)
    groups.append({'group': i, 'first_id': first, 'count': count, 'kind': kind, 'bank_bytes': size,
                   'file': FILES[i] if i < 23 else 'voice bank of voice row %d (player +0x2C, roster byte 6; 344 + 3*slot)' % (i - 23),
                   'samples': [dict(zip(('spu_offset', 'pitch', 'volume', 'channel'),
                                        struct.unpack_from('<IHBB', d, p - V + 8 * j))) for j in range(count)]})

voices = {}  # by voice row
for b in range(39):
    p, a, _, c, _, cool, e = struct.unpack_from('<IhhhhHH', d, 0x501808 - V + 16 * b)
    n = sh(p, 4)
    ids = sh(p + 8, sum(n))
    voices[b] = {'cooldown_frames': cool, 'by_type': {t + 2: ids[sum(n[:t]):sum(n[:t + 1])] for t in range(4)}}

out = {
    'round_state_jump_table_0x500420': [hex(x) for x in struct.unpack_from('<7I', d, 0x500420 - V)],
    'motions': motions,
    'announcer_round_numbers_0x500458': [hex(x) for x in h(0x500458, 9)],
    'sample_groups_0x42EBD8': groups,
    'voices_0x501808': voices,
    'hit_sounds_by_damage_0x500D10': {'female_bodies_5_7_10_18': [[hex(x) for x in h(a, 4)] for a in range(0x500D10, 0x500D30, 8)],
                                      'others': [[hex(x) for x in h(a, 4)] for a in range(0x500D30, 0x500D50, 8)]},
    'floor_sounds_by_stage_0x5007F8': [h(0x5007F8 + 6 * s, 3) for s in range(21)],
    'stage_select_0x440770': [{'name': name(struct.unpack_from('<I', d, a - V)[0]), 'stage': d[a - V + 4]}
                              for a in range(0x440770, 0x440818, 8)],
    'bgm_by_stage_0x4FB9C0': sh(0x4FB9C0, 24),
    # what 0x397A00 sends RSPU2DRV.IRX at boot: kind, s8, u16, s16, file of TEKKEN.BIN, loop sector
    'bgm_files_0x50CD00': [list(struct.unpack_from('<BbHhHh', d, 0x50CD00 - V + 10 * k)) for k in range(28)],
    'bgm_names_0x455D80': {struct.unpack_from('<b', d, a - V)[0]: name(struct.unpack_from('<I', d, a - V + 4)[0])
                           for a in range(0x455D80, 0x455D80 + 8 * 21, 8)},
}
json.dump(out, sys.stdout, indent=1)
