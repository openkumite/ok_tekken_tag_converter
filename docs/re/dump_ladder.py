# dump_ladder.py — arcade ladder tables: rival teams per character (0x51F3C8), fallback (0x51F560), CPU pairs (0x5207F0)
import struct, sys
V = 0x33F000
d = open(sys.path[0] + '/disc/SLUS_200.01', 'rb').read()
N = 'PAUL LAW LEI KING YOSHIMITSU NINA HWOARANG XIAOYU EDDY JIN JULIA KUMA BRYAN HEIHACHI OGRE MOKUJIN GUNJACK ROGER ANNA KAZUYA TRUE_OGRE JACK-2 P.JACK MICHELLE WANG LEE ARMOR_KING GANRYU KUNIMITSU DEVIL JUN BRUCE BAEK UNKNOWN'.split() + ['any']
n = lambda c: N[c] if 0 <= c < len(N) else str(c)
print('rival candidates per player character (stage 7), first free wins:')
for c in range(34):
    print(' ', n(c), [n(x) for x in struct.unpack_from('<3i', d, 0x51F3C8 - V + 12 * c)])
print('fallback:', [n(x) for x in struct.unpack_from('<4i', d, 0x51F560 - V)])
print('pairs (0x5207F0, 76 x 5 bytes: a, mask, b, mask, effect):')
for i in range(76):
    a, am, b, bm, e = struct.unpack_from('<bBbBb', d, 0x5207F0 - V + 5 * i)
    print('  %s+%s' % (n(a), n(b)), end='')
print()
