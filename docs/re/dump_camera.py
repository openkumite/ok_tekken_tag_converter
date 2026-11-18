# Camera tables of FIGHT_DATA.md section 18 as JSON: python3 dump_camera.py > camera.json
# Reads disc/SLUS_200.01 through ttt.py. Addresses are memory addresses.
import json, struct
from ttt import d, V

def h(a, n): return list(struct.unpack_from('<%dh' % n, d, a - V))

def move_lists():
    """0x434C78: per camera id (move flags >> 23, < 0x134) {u32 list, s16 count, s16 total, s16 split, s16 first mirrored}.
    List entries {u16 flags, s16 weight, s16 value, s16 value when mirrored}; weights become cumulative /4096 at boot
    (0x393898). value < 30: situation type value+1; 30..561: camera motion value-29; -1: none."""
    out, seen = {}, {}
    for i in range(0x134):
        p, cnt = struct.unpack_from('<Ih', d, 0x434C78 + 12 * i - V)
        if not p or cnt <= 0: continue
        if p in seen: out[hex(i)] = {'same_as': seen[p]}; continue
        seen[p] = hex(i)
        L = [struct.unpack_from('<Hhhh', d, p + 8 * k - V) for k in range(cnt)]
        w = sum(e[1] for e in L) or 1
        out[hex(i)] = [{'flags': f, 'percent': round(100 * c / w), 'value': a, 'mirrored': b} for f, c, a, b in L]
    return out

def motion_offsets():
    """0x508130: u32 offsets of the camera motions in disc file 12 (485 motions)."""
    o = struct.unpack_from('<600I', d, 0x508130 - V)
    n = next(i for i in range(1, 600) if o[i] <= o[i - 1])
    return list(o[:n])

def main():
    out = {
        'arena_projection': {  # 0x507FE0, 25 x {H, H min, H max, dolly flag, variant}
            'address': '0x507FE0', 'rows': [h(0x507FE0 + 10 * i, 5) for i in range(25)]},
        'situation_types': {  # 0x508908, 30 x {in frames, hold, pitch[4] (<<5), yaw offset[4] (<<6)}
            'address': '0x508908', 'rows': [h(0x508908 + 20 * i, 10) for i in range(30)]},
        'move_camera_lists': move_lists(),
        'camera_motion_offsets': motion_offsets(),
        'intro_offset': h(0x509C20, 7),  # eye x, y, z, target x, y, z, -
        'demo_programs': [list(struct.unpack_from('<20i', d, 0x433010 + 0x50 * i - V)) for i in range(10)],
        'replay_shots': {  # 0x520C40, 42 x {target, follow, pitch, yaw, roll, pitch rnd, yaw rnd, roll rnd, dist, dist rnd}
            'address': '0x520C40', 'rows': [h(0x520C40 + 20 * i, 10) for i in range(42)]},
        'replay_sequences': [list(d[0x520F88 - V + 4 * i:0x520F88 - V + 4 * i + 4]) for i in range(14)],
        'yaw_caps': {'0x13': 0x200, '0x14': 0x180, '0x15': 0x80, '0x16': 0x40},  # code 0x38F6B0, x 0xDDD0 / 256
    }
    assert len(out['camera_motion_offsets']) == 485, len(out['camera_motion_offsets'])
    assert out['arena_projection']['rows'][0][:3] == [615, 600, 900]
    print(json.dumps(out, indent=1))

if __name__ == '__main__':
    main()
