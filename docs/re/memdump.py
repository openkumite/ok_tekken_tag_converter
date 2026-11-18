# memdump.py START END — words of the ELF's data as hex, with printable runs shown as strings
import struct, sys
V = 0x33F000
d = open(sys.path[0] + '/disc/SLUS_200.01', 'rb').read()
a, b = int(sys.argv[1], 16), int(sys.argv[2], 16)
o = a - V
while o < b - V:
    s = d[o:o + 64].split(b'\0')[0]
    if len(s) >= 3 and all(32 <= c < 127 or c == 10 for c in s):
        print('%08x  "%s"' % (o + V, s.decode()))
        o += len(s) + 1; o = (o + 3) & ~3; continue
    w = struct.unpack_from('<I', d, o)[0]
    print('%08x  %08x  %s' % (o + V, w, ' '.join('%d' % x for x in struct.unpack_from('<hh', d, o))))
    o += 4
