# dump_bowl.py — Tekken Bowl's tables from SLUS_200.01 (memory = file + 0x33F000, gp = 0x5FD0F0):
# per-character bowler stats, voices, motions, the derived meter speeds, the pin rack and the physics constants.
import struct, sys
V, GP = 0x33F000, 0x5FD0F0
d = open(sys.path[0] + '/disc/SLUS_200.01', 'rb').read()
f32 = lambda a: struct.unpack_from('<f', d, a - V)[0]
gpf = lambda off: f32(GP - off)
NAMES = ('Paul Law Lei King Yoshimitsu Nina Hwoarang Xiaoyu Eddy Jin Julia Kuma Bryan Heihachi Ogre Mokujin '
         'Gun_Jack Roger Anna Kazuya True_Ogre Jack-2 P.Jack Michelle Wang Lee Armor_King Ganryu Kunimitsu Devil '
         'Jun Bruce Baek Unknown').split()

def meters(s, jack=False):
    # 0x3E9C58: S' compresses stats above 400; the spin meter speed and the power meter speed come from S'
    s2 = s if s <= 400 else 400 + (s - 400) * gpf(0x776C)
    spin = s2 * (gpf(0x7768) if jack else gpf(0x7764)) / 200 + gpf(0x7760)
    return spin, 2 * (s2 + 180) / 340

print('| Id | Character | S (0x523618) | hand | style | spin speed /frame | power speed /frame | '
      'motions (0x523590) | voices (0x5236A0) |')
print('|---|---|---|---|---|---|---|---|---|')
for c in range(34):
    s, left, fem = struct.unpack_from('<hBB', d, 0x523618 - V + 4 * c)
    mot = d[0x523590 - V + 4 * c:0x523590 - V + 4 * c + 4]
    voc = struct.unpack_from('<3H', d, 0x5236A0 - V + 8 * c)
    sp, pw = meters(s, c in (16, 21))
    print('| %d | %s | %d | %s | %s | %.3f | %.2f | %s | %s |' % (
        c, NAMES[c].replace('_', ' '), s, 'left' if left else 'right', 'female' if fem else '', sp, pw,
        ' '.join('0x%X' % m for m in mot), ' '.join('0x%X' % v for v in voc)))
print()
print('Win pose by moveset (0x5237D8):', [struct.unpack_from('<h', d, 0x5237D8 - V + 2 * i)[0] for i in range(33)])
print('Moveset -> character (0x5237B0):', list(d[0x5237B0 - V:0x5237B0 - V + 33]))
print()
# 0x3EA050: pin i's (x, z); y = gp-0x7758; the rack is shifted by z + 8000
rack = [(0, 0x7754), (0x7750, 0x774C), (0x7748, 0x7744), (0x7740, None), (None, None), (0x773C, None),
        (0x7738, 0x7728), (0x7734, 0x7728), (0x7730, 0x7728), (0x772C, 0x7728)]
for i, (x, z) in enumerate(rack):
    print('pin %d: x %7.1f  z %7.1f  y %.2f' % (i + 1, gpf(x) if x else 0.0, (gpf(z) if z else 0.0) + 8000,
                                               gpf(0x7758)))
print()
for off, what in ((0x7714, 'spin meter bound (Jack movesets 0x10, 0x13)'), (0x775C, 'ball start z'),
                  (0x7680, 'ball fall: y += (in air, per frame)'), (0x767C, 'ball fall: vy += per frame'),
                  (0x7674, 'ball bounce: vy *='), (0x7670, 'hook: lane start z'), (0x766C, 'hook: lane length'),
                  (0x7638, 'pin gravity'), (0x7620, 'pin base-contact radius^2'), (0x75D4, 'deck walls from z'),
                  (0x75D0, 'deck wall x'), (0x7548, 'roll over past z'), (0x7544, 'spectator hit radius^2'),
                  (0x771C, 'speed display factor')):
    print('gp-0x%X  %-45s %g' % (off, what, gpf(off)))
