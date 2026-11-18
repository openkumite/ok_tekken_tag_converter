# dump_rules.py — the fight-rule tables decoded in FIGHT_DATA.md sections 7-12 (jump tables, air reactions, anger
# relations, life / timer / rounds constants) and the corrected usage counts of conditions and transition types.
# python3 dump_rules.py > rules.json
import json, struct, collections, sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ttt
V, d = ttt.V, ttt.d
u32 = lambda a: ttt.u32(a - V)
def jt(a, n): return [hex(u32(a + 4 * i)) for i in range(n)]
def air(i):
    """air reaction entry (28 bytes): index < 8 at 0x500678, else 0x5F2BA8 + 28*index (0x374010)"""
    base = 0x500678 if i < 8 else 0x5F2BA8
    w = struct.unpack_from('<14h', d, base - V + 28 * i)
    return {'alias': w[0] & 0xffff, 'target_height': w[1], 'fixed_vy': w[2], 'w3': w[3], 'w4': w[4], 'w5': w[5],
            'samples': list(w[6:])}
out = {
    'condition_jump_table_0x4FF8E0': jt(0x4FF8E0, 0x60),
    'transition_jump_table_0x4FFA80': jt(0x4FFA80, 46),
    'transition_remap_0x4FFB40_types_2_25': jt(0x4FFB40, 24),
    'facing_mode_jump_table_0x500570': jt(0x500570, 15),
    'air_reactions': {i: air(i) for i in range(8)},
}
by2f = {}
for m in range(ttt.NMOVES):
    k = d[ttt.MOVES + 52 * m + 0x2F]
    if k: by2f[m] = {'air_index': k, **air(k)}
out['move_byte_2F_custom_air_reaction'] = by2f
out['life_options_0x51F5A0'] = [u32(0x51F5A0 + 4 * i) >> 12 for i in range(8)]
out['life_fixed_mode9_0x51F5B0'] = u32(0x51F5B0) >> 12
out['life_option_index_set_by_0x3B1488'] = 3
out['round_timer_frames_by_option'] = [0x870, 0xB04, 0x10A4, 0x1608, 0x1B30, None]  # 0x36D640; option 5 = no countdown
out['round_timer_display_seconds'] = [(t * 6 // 7 - 1) // 60 for t in (0x870, 0xB04, 0x10A4, 0x1608, 0x1B30)]  # 0x3C30AC
rel = []
for i in range(76):  # 0x3CF278: {partner body, variant mask, active body, variant mask, effect}; body 0x22 = any
    b = struct.unpack_from('<5b', d, 0x5207F0 - V + 5 * i)
    rel.append({'partner_body': b[0] & 0xff, 'partner_mask': b[1] & 0xff, 'active_body': b[2] & 0xff, 'active_mask': b[3] & 0xff,
                'effect': {-1: 'minus_delta', 1: 'plus_delta', -9: 'no_anger'}.get(b[4], b[4])})
out['anger_relations_0x5207F0'] = rel
cc, tc, fl = collections.Counter(), collections.Counter(), collections.Counter()
def entries():
    for i in range(ttt.NCANCEL): yield ttt.cancel(i)
    for i in range(ttt.NSHARED): yield ttt.cancel(i, ttt.SHARED)
for c in entries():
    if c['cmd'] in (0x8000, 0x8004, 0x8005): continue
    cc[hex(c['cond'])] += 1; t = c['trans']
    tc[t & 0x7F] += 1                       # type = trans & 0x7F (0x368CB0)
    for b, n in ((0x8000, 'turn'), (0x4000, '17E'), (0x2000, 'swap_or_moveset')): fl[n] += bool(t & b)
    fl['param_7_12_nonzero'] += bool(t & 0x1F80)
out['cond_usage'] = dict(sorted(cc.items(), key=lambda kv: int(kv[0], 16)))
out['trans_type_usage'] = dict(sorted(tc.items())); out['trans_flag_usage'] = dict(fl)
print(json.dumps(out, indent=1))
