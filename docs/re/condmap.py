# condmap.py — usage counts of cancel conditions / transition types in Tekken Tag, and how they pair with Tekken 3's
# (cancel lists of paired moves compared entry by entry where both lists have the same commands).
import collections, sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ttt, t3match
def all_entries():
    for i in range(ttt.NCANCEL): yield ttt.cancel(i)
    for i in range(ttt.NSHARED): yield ttt.cancel(i, ttt.SHARED)
cc, tc = collections.Counter(), collections.Counter()
for c in all_entries():
    if c['cmd'] in (0x8000, 0x8004, 0x8005): continue
    cc[c['cond']] += 1; tc[c['trans'] & 0x7f] += 1
print('cond usage (raw byte, with 0x80):'); print(' '.join('%02x:%d' % kv for kv in sorted(cc.items())))
print('trans usage:'); print(' '.join('%d:%d' % kv for kv in sorted(tc.items())))
cp, tp = collections.Counter(), collections.Counter()
for ms in range(14):
    T, P = t3match.pairs(ms, ms)
    E = T['cancels']['entries'] if isinstance(T['cancels'], dict) else T['cancels']
    for m3, i in P:
        a = []; k = m3['cancel_start']
        while True:
            e = E[k]; a.append(e); k += 1
            if e['kind'] == 'end': break
        b = ttt.cancel_list(ttt.move(i)[3])
        if len(a) != len(b): continue
        for x, y in zip(a, b):
            if x['kind'] != 'command' or y['cmd'] >= 0x8000: continue
            cp[(x['cond'], y['cond'])] += 1; tp[(x['transition_type'], y['trans'] & 0x7f)] += 1
print('T3 cond -> TTT cond (paired):'); print(' '.join('%02x>%02x:%d' % (a, b, n) for (a, b), n in sorted(cp.items()) if a != b or a > 0x23))
print('T3 trans -> TTT trans (paired):'); print(' '.join('%d>%d:%d' % (a, b, n) for (a, b), n in sorted(tp.items())))
