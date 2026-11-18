# dump_fighter.py NAME [out.json] — everything decoded for one fighter (roster name, e.g. PAUL, JIN) as JSON.
import sys, json, struct, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ttt
d, V = ttt.d, ttt.V
DIRS = {1: 'd/b', 2: 'd', 3: 'd/f', 4: 'b', 5: 'n', 6: 'f', 7: 'u/b', 8: 'u', 9: 'u/f'}
SPECIAL = {0x8000: 'END', 0x8001: 'dash', 0x8002: 'backdash', 0x8003: 'special_8003', 0x8004: 'CALL', 0x8005: 'RETURN'}
SEQ = 0x4EFD44 - V  # sequence of cmd c (>= 0x8006) is at SEQ + 2*c: s16 frame window, steps (buttons<<8 | numpad dir), 0
def buttons(b):
    s = '+'.join(str(k + 1) for k in range(4) if b >> k & 1)
    if b & 0x4000 or b & 0x40: s = (s + '+' if s else '') + 'TAG'
    return s
def command(c):
    if c in SPECIAL: return SPECIAL[c]
    if c >= 0x8006:
        o = SEQ + 2 * c; steps = []; o += 2
        while ttt.u16(o):
            v = ttt.u16(o); b = v >> 8
            steps.append(DIRS.get(v & 15, '?') + ('+' + buttons(b & 0x4f) if b & 0x4f else '')); o += 2
        return ','.join(steps)
    ds = '|'.join(DIRS[k] for k in range(1, 10) if c >> (4 + k) & 1)
    b = buttons(c & 0x400f)
    return (ds + ('+' if ds and b else '') + b) + (' (held/any)' if c & 0x10 and b else '') or 'always'
def req(r):
    for lo, hi, n in ((1, 0x21, 'moveset_is'), (0x22, 0x42, 'moveset_not'), (0x43, 0x63, 'opponent_moveset_is'), (0x64, 0x84, 'partner_moveset_is'),
                      (0x85, 0xa6, 'body_is'), (0xa7, 0xc8, 'partner_body_is'), (0xc9, 0xea, 'body_not')):
        if lo <= r <= hi: return {'kind': n, 'value': r - lo}
    return {'kind': {0: 'any', 0xeb: 'flag_418', 0xec: 'kazuya_devil_pair', 0xed: 'not_kazuya_devil_pair'}.get(r, 'any?'), 'raw': r}
def cancels(start, base=ttt.CANCELS, end=(0x8000,)):
    out = []
    for c in ttt.cancel_list(start, base, end):
        c = dict(c); c['input'] = command(c['cmd']); c['req_decoded'] = req(c['req'])
        c['turn_180'] = bool(c['trans'] & 0x8000); c['trans_bit14'] = bool(c['trans'] & 0x4000); c['trans_type'] = c['trans'] & 0x3fff
        c['tag_cond'] = bool(c['cond'] & 0x80); c['cond'] &= 0x7f; c['reversal'] = c['cond'] >= 0x60
        out.append(c)
    return out
def hits(i):
    out = []
    while True:
        g, dmg, react, close, dist = struct.unpack_from('<5H', d, ttt.HIT10 + 10 * i)
        out.append({'gate': None if g == 0xffff else req(g), 'damage': dmg & 0x3fff, 'damage_type': dmg >> 14, 'reaction': react,
                    'close_reaction': close, 'close_distance': dist}); i += 1
        if g == 0xffff: return out
def props(i):
    out = []; o = ttt.PROPS + 4 * i
    while ttt.u16(o): out.append({'frame': ttt.u16(o), 'op': ttt.u16(o + 2) >> 8, 'arg': ttt.u16(o + 2) & 0xff}); o += 4
    return out
def sounds(i):
    out = []; o = ttt.SOUNDS + 2 * i
    while ttt.u16(o) != 0xffff: out.append({'type': ttt.u16(o) >> 12, 'index': ttt.u16(o) & 0xfff}); o += 2
    return out
def move(i, F):
    w = ttt.move(i); b = d[ttt.MOVES + 52 * i:ttt.MOVES + 52 * i + 52]
    return {'move': i, 'motion': w[0], 'frames': F[i],
            'posture': w[1], 'base_posture': w[1] & 7, 'guard': w[1] & 0x18, 'state_code': w[1] >> 11 & 31, 'tag_state': bool(w[1] >> 24 & 1),
            'attack_mask': w[2], 'level': w[2] >> 8 & 0xff,
            'cancel_start': w[3], 'cancels': cancels(w[3]),
            'next_alias': w[4] & 0xffff, 'next_turn': w[4] >> 16,
            'hit_index': w[5] & 0xffff, 'hit': hits(w[5] & 0xffff), 'air_drift': struct.unpack_from('<h', b, 0x16)[0],
            'jump_start': b[0x19], 'jump_end': b[0x1a], 'hold_frame': b[0x1b],
            'sounds': None if w[7] == 0xffffffff else sounds(w[7]),
            'properties': None if w[8] == 0xffffffff else props(w[8]),
            'flags': w[9] & 0xffffff, 'camera_script': w[9] >> 24,
            'bones': {'end0': b[0x28], 'start0': b[0x29], 'end1': b[0x2a], 'start1': b[0x2b]},
            'active': [b[0x2c], b[0x2d]], 'hitstop': struct.unpack_from('<b', b, 0x2e)[0], 'byte_2f': b[0x2f],
            'push_control': w[12] & 0xffff}
def fighter(name):
    r = ttt.roster()[name.upper()]; ms, body = r['moveset'], r['body']; F = ttt.frames()
    al = [ttt.alias(ms, a) for a in range(ttt.NALIAS)]
    base = body if body < 34 else {34: 8, 35: 11, 36: 15, 37: 17, 38: 29}[body]  # costume bodies use their host's tables
    ptr = lambda t: struct.unpack_from('<I', d, t - V + 4 * base)[0] - V
    return {'name': name.upper(), 'body': body, 'moveset': ms,
            'health_percent': struct.unpack_from('<I', d, 0x51F5C0 - V + 4 * base)[0],
            'hurt_radii': list(struct.unpack_from('<14h', d, ptr(0x42F098))), 'hurt_bones': list(struct.unpack_from('<14I', d, 0x42F060 - V)),
            'push_radii': list(struct.unpack_from('<8h', d, ptr(0x42EFD8))), 'push_bones': list(struct.unpack_from('<8I', d, 0x42EFB8 - V)),
            'aliases': [None if a == 7350 else a for a in al],
            'moves': [move(i, F) for i in sorted(set(al) - {7350})]}
if __name__ == '__main__':
    f = fighter(sys.argv[1]); out = sys.argv[2] if len(sys.argv) > 2 else '%s/out/%s.json' % (ttt.HERE, sys.argv[1].lower())
    # self-check: every list ends, every alias resolves, hit chains end
    assert all(m['cancels'][-1]['cmd'] == 0x8000 for m in f['moves']) and all(m['hit'][-1]['gate'] is None for m in f['moves'])
    json.dump(f, open(out, 'w')); print(out, len(f['moves']), 'moves', os.path.getsize(out), 'bytes')
