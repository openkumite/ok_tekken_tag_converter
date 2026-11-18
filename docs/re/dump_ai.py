# dump_ai.py — the tables of FIGHT_DATA.md sections 19-20: CPU AI profiles (5 difficulties x 8 levels), the
# practice profile, the survival ladder, range thresholds per body, zone masks, per-moveset AI handlers, the
# options' descriptors and defaults. python3 dump_ai.py > ai.json
import json, struct, sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ttt
V, d = ttt.V, ttt.d
def u(fmt, a): return list(struct.unpack_from('<' + fmt, d, a - V))
def name(p): return d[p - V:d.index(b'\0', p - V)].decode('latin1').strip()

DIFF = ['EASY', 'NORMAL', 'HARD', 'VERY HARD', 'ULTRA HARD']
def profile(a):  # 128 bytes: 4 u8 then 62 s16 at +4 (Tekken 3's 110-byte profile with 4 bytes inserted at +4)
    return {'wake_tap': u('4B', a), 's16_from_04': u('62h', a + 4)}
profiles = {DIFF[k]: [profile(0x431918 + 0x400 * k + 0x80 * lv) for lv in range(8)] for k in range(5)}
practice = profile(0x432D90)
assert profiles['EASY'][0]['s16_from_04'][3:7] == [300, 300, 400, 400]   # attack cooldown choices +0x0A..+0x10

n = u('i', 0x5FD0F0 - 0x6FC0)[0]                                          # gp-0x6FC0 = 15 rows
ladder = [dict(zip(('max_wins', 'difficulty', 'level_lo', 'level_hi'), u('4H', 0x432D18 + 8 * i))) for i in range(n)]
ranges = [u('6H', 0x432E10 + 12 * b) for b in range(40)]                  # per body (+0x2A)
zones = {'my_reach': u('6I', 0x4318D8), 'his_reach': u('6I', 0x4318F0)}

def fn(a):
    if a == 0: return None
    return 'stub' if u('I', a)[0] == 0x03E00008 else hex(a)
handlers = {}
for i in range(34):
    ms, vs, own = u('3I', 0x507A80 + 12 * i)
    handlers[ms] = {'vs_this_moveset': fn(vs), 'own': fn(own)}

options = []
for k in range(12):
    w = u('16I', 0x439CE0 + 0x40 * k)
    vals = [name(x) for x in w[4:10] if x]
    options.append({'config': hex(w[0]), 'config_offset': hex(w[0] - 0x439C60), 'default': w[1],
                    'name': name(w[2]), 'values': vals,
                    'default_text': vals[w[1]] if w[1] < len(vals) else w[1]})
assert options[2]['name'] == 'ROUND TIME' and options[2]['default_text'] == '60'

json.dump({'profiles': profiles, 'practice_profile': practice, 'survival_ladder': ladder, 'ranges': ranges,
           'zone_masks': zones, 'moveset_handlers': handlers, 'options': options,
           'ai_lcg_seed': u('I', 0x5FD0F0 - 0x64E8)[0]}, sys.stdout, indent=1)
