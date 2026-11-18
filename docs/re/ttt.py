# Shared loaders for the Tekken Tag (SLUS-20001) fight tables. Offsets are file offsets; memory = file + 0x33F000.
import struct, json, os, sys
HERE = os.path.dirname(os.path.abspath(__file__))
V = 0x33F000
d = open(HERE + '/disc/SLUS_200.01', 'rb').read()
MOVES, NMOVES = 0x1F2EA0, 7351                 # 52-byte records
ALIASES, NALIAS, NSETS = 0x2503D0, 5532, 33    # u16 move per alias, 7350 = missing
CANCELS, NCANCEL = 0x143248, 0x82CF            # 14-byte entries (mem 0x482248)
SHARED, NSHARED = 0x140780, 782                # shared cancel lists (mem 0x47F780)
PROPS = 0x524690 - V                           # timed properties, u16 frame, u16 op<<8|arg (record +0x20 = index of u32)
SOUNDS = 0x52C448 - V                          # sound lists, u16 entries (record +0x1C = index)
HIT10 = 0x5E8978 - V                           # 10-byte records indexed by record +0x14 low half
MOTLISTS = 0x5E8608 - V                        # s16 motion lists, -1 terminated (reversal cond)
ROSTER = 0x1BCC20
u16 = lambda o: struct.unpack_from('<H', d, o)[0]
u32 = lambda o: struct.unpack_from('<I', d, o)[0]
def move(i): return struct.unpack_from('<13I', d, MOVES + 52 * i)
def alias(ms, a): return u16(ALIASES + 2 * (ms * NALIAS + a))
def cancel(i, base=CANCELS):
    k = ('cmd', 'req', 'cond', 'param', 'dest', 'trans', 'b10', 'win_start', 'win_end', 'exec')
    return dict(zip(k, struct.unpack_from('<HBBHHHBBBB', d, base + 14 * i)))
def cancel_list(start, base=CANCELS, end=(0x8000,)):
    out = []
    while True:
        c = cancel(start, base); out.append(c); start += 1
        if c['cmd'] in end: return out
def frames():
    """frame count of each motion (byte 0 of the motion), from the research repo's raw motion banks"""
    p = HERE + '/frames.json'
    if os.path.exists(p): return json.load(open(p))
    r = os.environ.get('TTT_RAW', 'raw/')
    a, b = open(r + '133.bin', 'rb').read(), open(r + '010.bin', 'rb').read()
    out = []
    for i in range(NMOVES):
        o, = struct.unpack_from('<I', a, 8 * i)
        out.append(b[o - 0x4000000] if o >= 0x4000000 else a[o])
    json.dump(out, open(p, 'w')); return out
def roster():
    out = {}
    for r in range(ROSTER, ROSTER + 16 * 50, 16):
        p = u32(r)
        if not (0x340000 <= p < 0x600000): continue
        name = d[p - V:d.index(b'\0', p - V)].decode('latin1')
        out[name.upper()] = {'body': d[r + 4], 'moveset': d[r + 7], 'bytes': list(d[r + 4:r + 16])}
    return out
