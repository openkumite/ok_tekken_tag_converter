# uploads.py FILE.bin — the GS uploads of a 2D art file in order (the game's sprite order, FRONTEND.md 1):
# index, buffer base/width, format, destination x/y, width x height. A sprite id = group first id + index.
import struct, sys
d = open(sys.argv[1], 'rb').read(); o = 0; i = 0; PSM = {0: 'CT32', 1: 'CT24', 2: 'CT16', 0x13: 'T8', 0x14: 'T4'}
while o + 16 <= len(d):
    lo, hi = struct.unpack_from('<QQ', d, o); nloop = lo & 0x7FFF; flg = (lo >> 58) & 3; nreg = (lo >> 60) or 16
    if nloop == 0: break
    if flg == 0:  # A+D: BITBLTBUF, TRXPOS, TRXREG, TRXDIR
        r = {}
        for k in range(nloop * nreg):
            v, a = struct.unpack_from('<QQ', d, o + 16 + 16 * k); r[a & 0xFF] = v
        b, p, g = r.get(0x50, 0), r.get(0x51, 0), r.get(0x52, 0)
        q = o + 16 + 16 * nloop * nreg; img = struct.unpack_from('<Q', d, q)[0]; n = img & 0x7FFF
        print('%3d dbp=%04x dbw=%2d %-4s x=%4d y=%4d %4dx%-4d' % (i, (b >> 32) & 0x3FFF, (b >> 48) & 0x3F, PSM.get((b >> 56) & 0x3F, hex((b >> 56) & 0x3F)),
              (p >> 32) & 0x7FF, (p >> 48) & 0x7FF, g & 0xFFF, (g >> 32) & 0xFFF))
        o = q + 16 + 16 * n; i += 1
    else:
        o += 16 + 16 * nloop
