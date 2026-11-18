# extract_file.py DISC.zip OUTDIR N [N...] — TEKKEN.BIN file N (first sector unscrambled) as OUTDIR/NNN.bin (FORMATS.md 2)
import zipfile, struct, sys, os
V = 0x33F000
elf = open(os.path.join(sys.path[0], 'disc', 'SLUS_200.01'), 'rb').read()
z = zipfile.ZipFile(sys.argv[1]); f = z.open([n for n in z.namelist() if n.endswith('.bin')][0])
def sec(n):
    f.seek(n * 2352 + 24); return f.read(2048)
pvd = sec(16); lba, size = struct.unpack_from('<I4xI', pvd[156:190], 2)
d = b''.join(sec(lba + i) for i in range((size + 2047) // 2048)); o = 0; base = None
while o < len(d):
    l = d[o]
    if l == 0: o = (o // 2048 + 1) * 2048; continue
    if d[o + 33:o + 33 + d[o + 32]].decode('latin1').startswith('TEKKEN.BIN'): base = struct.unpack_from('<I', d, o + 2)[0]
    o += l
for n in map(int, sys.argv[3:]):
    s, sz, _, idx = struct.unpack_from('<4I', elf, 0x1CAE40 + 16 * n)
    s -= 0x4820; sz -= 0x350A
    data = bytearray(b''.join(sec(base + s + i) for i in range((sz + 2047) // 2048))[:sz])
    if idx != 0xFFFF and n != 44:
        key = (218319 * (n + 0xF3C) + (n - 0x348F) * 0x1234567) & (2**64 - 1)
        for w in range(0, min(2048, sz) & ~3, 4):
            struct.pack_into('<I', data, w, struct.unpack_from('<I', data, w)[0] ^ (key & 0xFFFFFFFF))
            key = (key * 0x71C3AB9 + 0x3941) & (2**64 - 1)
    open(os.path.join(sys.argv[2], '%03d.bin' % n), 'wb').write(data); print(n, sz)
