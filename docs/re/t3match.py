# Pair Tekken 3 moves with Tekken Tag moves of the same moveset by (frames, attack mask, active frames); used by the validation scripts.
import json, collections, struct, sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ttt
G = os.path.expanduser('~/Library/Application Support/OpenKumite/assets/gamedata/movesets/')
F = ttt.frames()
def t3(ms): return json.load(open(G + 'moveset_%02d.json' % ms))
def pairs(t3ms, tttms):
    """[(t3 move dict, ttt move number)] for attacks whose key is unique on both sides"""
    T = t3(t3ms)
    mine = sorted({ttt.alias(tttms, a) for a in range(ttt.NALIAS)} - {7350})
    key3 = lambda m: (m['frames'], m['attack_mask']['raw'] & 0xffff, m['active'][0], m['active'][1])
    def keyt(i):
        w = ttt.move(i); return (F[i], w[2] & 0xffff, w[11] & 0xff, (w[11] >> 8) & 0xff)
    a, b = collections.defaultdict(list), collections.defaultdict(list)
    for m in T['moves']:
        if m['attack_mask']['raw'] and m['active'][0]: a[key3(m)].append(m)
    for i in mine:
        if ttt.move(i)[2]: b[keyt(i)].append(i)
    return T, [(a[k][0], b[k][0]) for k in a if len(a[k]) == 1 and len(b.get(k, [])) == 1]
