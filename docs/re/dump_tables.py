# dump_tables.py — the tables every fighter shares, to out/tables.json
import sys, json, struct, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ttt, dump_fighter as df
d, V = ttt.d, ttt.V
REACT, NREACT, CURVES = 0x46BE78 - V, 1429, 0x523FA0 - V
def reaction(i):
    p = struct.unpack_from('<7I', d, REACT + 56 * i); v = struct.unpack_from('<14H', d, REACT + 56 * i + 28)
    cur = lambda x: None if x == 0 else (x - 0x523FA0) // 24
    return {'index': i, 'pushback_angle': p[0], 'curve_hit': cur(p[1]), 'angle_2': p[2], 'curve_counter': cur(p[3]),
            'angle_grounded': p[4], 'curve_grounded': cur(p[5]), 'curve_block': cur(p[6]), 'victims': list(v)}
def curve(i):
    v = struct.unpack_from('<12h', d, CURVES + 24 * i); return {'constant_frames': v[0], 'velocity': v[1], 'impulses': list(v[2:])}
def motion_lists():
    out, cur, o = [], [], ttt.MOTLISTS
    while o < 0x5E8978 - V:
        v = struct.unpack_from('<h', d, o)[0]; o += 2
        if v == -1: out.append(cur); cur = []
        else: cur.append(v)
    return out
t = {'reaction_slots': ['front_standing', 'front_crouching', 'counter_standing', 'counter_crouching', 'side_a_standing', 'side_a_crouching',
                        'side_b_standing', 'side_b_crouching', 'back_standing', 'back_crouching', 'block_standing', 'block_crouching', 'grounded_faceup', 'grounded_facedown'],
     'reactions': [reaction(i) for i in range(NREACT)], 'reaction_none': 0x592,
     'pushback_curves': [curve(i) for i in range(72)],
     'shared_cancels': df.cancels(0, ttt.SHARED, ()) if False else [dict(ttt.cancel(i, ttt.SHARED), input=df.command(ttt.cancel(i, ttt.SHARED)['cmd'])) for i in range(ttt.NSHARED)],
     'reversal_motion_lists_flat': motion_lists(),
     'health_percent_by_body': list(struct.unpack_from('<34I', d, 0x51F5C0 - V)),
     'life_options': [x / 4096 for x in struct.unpack_from('<8I', d, 0x51F5A0 - V)],
     'recoverable_health_factor': struct.unpack_from('<f', d, 0x5F5168 - V)[0], 'recoverable_health_factor_flag_12a': struct.unpack_from('<f', d, 0x5F516C - V)[0],
     'roster': ttt.roster()}
json.dump(t, open(ttt.HERE + '/out/tables.json', 'w')); print('tables.json', os.path.getsize(ttt.HERE + '/out/tables.json'))
