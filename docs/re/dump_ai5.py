# dump_ai5.py — the tables of FIGHT_DATA.md section 21 (CPU AI, fifth pass): the profile fields the chain steps
# read (named, per difficulty and level), the canned sequences of the moveset handlers (King, Ogre, Ganryu), the
# sine table of the distance jitter and the live handler rows. python3 dump_ai5.py > ai5.json
import json, struct, sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ttt
V, d = ttt.V, ttt.d
def u(fmt, a): return list(struct.unpack_from('<' + fmt, d, a - V))

DIFF = ['EASY', 'NORMAL', 'HARD', 'VERY HARD', 'ULTRA HARD']
FIELDS = {  # profile offset -> name (section 21); the reader in brackets
    0x08: 'cooldown_base', 0x0A: 'cooldown_0', 0x0C: 'cooldown_1', 0x0E: 'cooldown_2', 0x10: 'cooldown_3',
    0x14: 'throw_attempt', 0x1A: 'king_canned_throw',
    0x20: 'guard', 0x22: 'guard_cap', 0x24: 'counter', 0x26: 'counter_cap', 0x28: 'sidestep',
    0x32: 'list_c_count', 0x34: 'list_c_prob', 0x36: 'list_b_count', 0x38: 'list_b_prob',
    0x3A: 'list_d_count', 0x3C: 'list_d_prob', 0x3E: 'handler_gate', 0x4A: 'always_zero', 0x4C: 'anti_air',
    0x50: 'keep_walk_back', 0x66: 'stun_tier3_0x1C', 0x68: 'stun_tier1_0x1E', 0x6E: 'tag_out', 0x70: 'stun_tier2_tag',
    0x72: 'guard_after_reaction', 0x74: 'delay_input', 0x76: 'repeat_count', 0x78: 'repeat_guard'}
def prof(k, lv):
    a = 0x431918 + 0x400 * k + 0x80 * lv
    return {n: u('h', a + o)[0] for o, n in FIELDS.items()}
profiles = {DIFF[k]: [prof(k, lv) for lv in range(8)] for k in range(5)}

def seq(a):  # step words, buttons << 8 | numpad direction, 0-terminated after the first word (0x3811E8)
    w = [u('H', a)[0]]
    while u('H', a + 2 * len(w))[0]: w.append(u('H', a + 2 * len(w))[0])
    return ['%d%s' % (x & 0xF, '+' + ''.join(str(b + 1) for b in range(4) if x & 0x100 << b) if x & 0xF00 else '') for x in w]
sequences = {
    'king_vs_standing': [seq(0x507A52 + 14 * i) for i in range(3)],
    'king_vs_crouching': [seq(0x507A32 + 14 * i) for i in range(2)],
    'king_vs_down': [seq(0x507A12 + 14 * i) for i in range(2)],
    'ganryu_vs_0x1407': seq(0x507618),
    'ogre_punish_pads': ['%#06x' % x for x in u('4H', 0x507A08)],   # pad words, one drawn by rand() & 3
    'devil_punish_pads': ['%#06x' % x for x in u('4H', 0x507A00)],
}
sine = u('4096h', 0x4FCE88)
handlers = {}
for i in range(34):
    ms, vs, own = u('3I', 0x507A80 + 12 * i)
    live = lambda a: a and u('2I', a) != [0x03E00008, 0x2402FFFF]   # not `jr ra; li v0, -1`
    if live(vs) or live(own):
        handlers[ms] = {'vs_this_moveset': hex(vs) if live(vs) else None, 'own': hex(own) if live(own) else None}

assert profiles['ULTRA HARD'][7]['cooldown_3'] == 150 and profiles['NORMAL'][0]['list_d_count'] == 1
assert sine[0] == 0 and sine[1024] == 4096 and sine[3072] == -4096
assert sequences['king_vs_standing'][0] == ['5', '6', '3', '3+13', '3']
assert set(handlers) == {0, 3, 4, 6, 14, 25, 27}
json.dump({'profiles': profiles, 'sequences': sequences, 'handlers': handlers}, sys.stdout, indent=1)
