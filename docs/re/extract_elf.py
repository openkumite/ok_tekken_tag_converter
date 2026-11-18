# Pull SLUS_200.01 out of the zipped Mode-2 CD image (2352-byte sectors, data at +24).
import zipfile, struct, sys
z = zipfile.ZipFile(sys.argv[1]); name = [n for n in z.namelist() if n.endswith('.bin')][0]
f = z.open(name); buf = bytearray()
def sec(n):  # sequential reader with cache of everything read so far (ELF is near the start)
    need = (n + 1) * 2352
    while len(buf) < need: buf.extend(f.read(max(need - len(buf), 1 << 22)))
    return bytes(buf[n * 2352 + 24:n * 2352 + 24 + 2048])
pvd = sec(16); root = pvd[156:190]; lba, size = struct.unpack_from('<I4xI', root, 2)
d = b''.join(sec(lba + i) for i in range((size + 2047) // 2048)); o = 0
while o < len(d):
    l = d[o]
    if l == 0: o = (o // 2048 + 1) * 2048; continue
    nm = d[o + 33:o + 33 + d[o + 32]].decode('latin1')
    if nm.startswith('SLUS_200.01'):
        fl, fs = struct.unpack_from('<I4xI', d, o + 2)
        data = b''.join(sec(fl + i) for i in range((fs + 2047) // 2048))[:fs]
        assert fs == 2873728 and data[:4] == b'\x7fELF'
        open(sys.argv[2], 'wb').write(data); print('ok', fl, fs)
    o += l
