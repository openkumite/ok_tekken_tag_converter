# dump_endings.py SLUS_200.01 [RAW_DIR] — Tekken Tag's endings: the ending table 0x51B678, the descriptors 0x51B488
# (script, arena, cast), each script as text, and (with RAW_DIR = the converted pack's raw/) the scene files' frame
# counts: camera path 2e+47, motion pack 2e+48 (see BOWL_ENDINGS.md).
import struct, sys, os
V = 0x33F000
d = open(sys.argv[1], 'rb').read()
raw = sys.argv[2] if len(sys.argv) > 2 else None
NAMES = ('Paul Law Lei King Yoshimitsu Nina Hwoarang Xiaoyu Eddy Jin Julia Kuma Bryan Heihachi Ogre Mokujin GunJack '
         'Roger Anna Kazuya TrueOgre Jack-2 P.Jack Michelle Wang Lee ArmorKing Ganryu Kunimitsu Devil Jun Bruce Baek '
         'Unknown').split()
OPS = {0: 'actor0 = cast', 1: 'actor1 = cast', 2: 'actor0 track', 3: 'actor1 track', 4: 'camera path, frame',
       5: 'flash (alpha b, over a)', 6: 'fade to white', 10: 'effect at joint', 12: 'actor show/hide',
       15: 'obj 0x3AC558', 16: 'actor placement 0x378368', 17: 'effect 0x3A2758', 18: 'effect 0x3A2758 (joint)',
       19: 'fade in from black', 20: 'fade out to black', 21: 'obj 0x3AC7C0', 22: 'obj 0x3AC8B0', 23: 'obj 0x3AC978',
       24: 'obj 0x3ACAB0', 25: 'obj 0x3ACB08', 26: 'flag gp-0x6E84', 27: 'clear objects', 28: 'Kuma/Panda +0xB052',
       29: 'obj 0x3AA4B8', 30: 'obj 0x3AD1D8', 31: 'call 0x34CB38 by bits', 32: 'obj 0x3AA410'}
u = lambda f, a: struct.unpack_from(f, d, a - V)

def frames(path, part=None):
    b = open(path, 'rb').read()
    if part is None: return struct.unpack_from('<I', b, 0)[0]
    n, o = struct.unpack_from('<II', b, 0)
    for i in range(part): o = struct.unpack_from('<I', b, 12 + 8 * i)[0]
    m, s = struct.unpack_from('<II', b, o)
    out = []
    for i in range(m):
        out.append(struct.unpack_from('<H', b, o + s)[0])
        s = struct.unpack_from('<I', b, o + 12 + 8 * i)[0]
    return out

print('character x costume -> ending (0x51B678)')
for c in range(34): print('%-10s' % NAMES[c], u('<8b', 0x51B678 + 8 * c))
for e in range(38):
    script, arena, _, cast = u('<IHHI', 0x51B488 + 12 * e)
    cl, o = [], cast
    while u('<h', o)[0] >= 0: cl.append(u('<hh', o)); o += 4
    line = 'ending %d: arena %d, cast %s, script 0x%X' % (e, arena,
           ', '.join('%s(%s)' % (NAMES[a], 'player' if b < 0 else b) for a, b in cl), script)
    if raw:
        cam = os.path.join(raw, '%03d.bin' % (2 * e + 47)); pk = os.path.join(raw, '%03d.bin' % (2 * e + 48))
        line += ', camera %03d: %d frames, pack %03d: %s' % (2 * e + 47, frames(cam), 2 * e + 48, frames(pk, 1))
    print(line)
    o = script
    while True:
        f, op, a, b, c = u('<5h', o); o += 10
        if op == -1: print('   %5d end' % f); break
        print('   %5d op %2d %-26s %d %d %d' % (f, op, OPS.get(op, 'no-op'), a, b, c))
