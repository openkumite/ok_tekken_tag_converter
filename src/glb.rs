//! Minimal GLB (glTF 2.0 binary) writer and the small matrix maths the exporters share.
use serde_json::{Value, json};

pub type M4 = [[f64; 4]; 4];
pub const ID: M4 = [[1., 0., 0., 0.], [0., 1., 0., 0.], [0., 0., 1., 0.], [0., 0., 0., 1.]];

// (the matrix maths indexes fixed 4 x 4 arrays within constant bounds)
#[allow(clippy::indexing_slicing)]
pub fn mul(a: &M4, b: &M4) -> M4 {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..4).map(|k| a[i][k] * b[k][j]).sum()))
}
#[allow(clippy::indexing_slicing)]
pub fn point(m: &M4, p: [f32; 3]) -> [f64; 3] {
    std::array::from_fn(|i| m[i][0] * p[0] as f64 + m[i][1] * p[1] as f64 + m[i][2] * p[2] as f64 + m[i][3])
}
#[allow(clippy::indexing_slicing)]
pub fn dir(m: &M4, p: [f32; 3]) -> [f64; 3] {
    std::array::from_fn(|i| m[i][0] * p[0] as f64 + m[i][1] * p[1] as f64 + m[i][2] * p[2] as f64)
}
/// Rz(z)·Ry(y)·Rx(x) with a translation: the engine's joint transform.
pub fn joint(r: [f64; 3], t: [f64; 3]) -> M4 {
    // (libm, not std: the same bits on every platform)
    let [(sx, cx), (sy, cy), (sz, cz)] = r.map(|a| (libm::sin(a), libm::cos(a)));
    [
        [cy * cz, sx * sy * cz - cx * sz, sx * sz + cx * sy * cz, t[0]],
        [cy * sz, cx * cz + sx * sy * sz, cx * sy * sz - sx * cz, t[1]],
        [-sy, sx * cy, cx * cy, t[2]],
        [0., 0., 0., 1.],
    ]
}
/// Inverse of a rigid transform.
#[allow(clippy::indexing_slicing)]
pub fn inv(m: &M4) -> M4 {
    let mut r = ID;
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = m[j][i];
        }
    }
    for row in r.iter_mut().take(3) {
        row[3] = -(0..3).map(|k| row[k] * m[k][3]).sum::<f64>();
    }
    r
}
/// Quaternion (x, y, z, w) of a rotation matrix.
pub fn quat(m: &M4) -> [f32; 4] {
    let t = m[0][0] + m[1][1] + m[2][2];
    let q = if t > 0. {
        let s = libm::sqrt(t + 1.) * 2.;
        [(m[2][1] - m[1][2]) / s, (m[0][2] - m[2][0]) / s, (m[1][0] - m[0][1]) / s, s / 4.]
    } else if m[0][0] > m[1][1] && m[0][0] > m[2][2] {
        let s = libm::sqrt(1. + m[0][0] - m[1][1] - m[2][2]) * 2.;
        [s / 4., (m[0][1] + m[1][0]) / s, (m[0][2] + m[2][0]) / s, (m[2][1] - m[1][2]) / s]
    } else if m[1][1] > m[2][2] {
        let s = libm::sqrt(1. + m[1][1] - m[0][0] - m[2][2]) * 2.;
        [(m[0][1] + m[1][0]) / s, s / 4., (m[1][2] + m[2][1]) / s, (m[0][2] - m[2][0]) / s]
    } else {
        let s = libm::sqrt(1. + m[2][2] - m[0][0] - m[1][1]) * 2.;
        [(m[0][2] + m[2][0]) / s, (m[1][2] + m[2][1]) / s, s / 4., (m[1][0] - m[0][1]) / s]
    };
    q.map(|v| v as f32)
}

pub const F32: u32 = 5126;
pub const U8: u32 = 5121;
pub const U16: u32 = 5123;
pub const U32: u32 = 5125;

#[derive(Default)]
pub struct Glb {
    pub bin: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
}

/// Sets a key of a JSON object.
pub fn set(object: &mut Value, key: &str, value: Value) {
    if let Some(o) = object.as_object_mut() {
        o.insert(key.into(), value);
    }
}

pub fn floats(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

impl Glb {
    pub fn view(&mut self, data: &[u8]) -> usize {
        while !self.bin.len().is_multiple_of(4) {
            self.bin.push(0);
        }
        self.views.push(json!({"buffer": 0, "byteOffset": self.bin.len(), "byteLength": data.len()}));
        self.bin.extend(data);
        self.views.len() - 1
    }
    pub fn accessor(&mut self, data: &[u8], comp: u32, ty: &str, count: usize) -> usize {
        let view = self.view(data);
        let mut a = json!({"bufferView": view, "componentType": comp, "type": ty, "count": count});
        if comp == U8 {
            set(&mut a, "normalized", json!(true)); // colours
        }
        self.accessors.push(a);
        self.accessors.len() - 1
    }
    /// A float accessor with min/max (required for POSITION and animation input).
    pub fn ranged(&mut self, v: &[f32], n: usize, ty: &str) -> usize {
        let a = self.accessor(&floats(v), F32, ty, v.len() / n.max(1));
        let (mut lo, mut hi) = (vec![f32::MAX; n], vec![f32::MIN; n]);
        for c in v.chunks(n.max(1)) {
            for ((lo, hi), x) in lo.iter_mut().zip(&mut hi).zip(c) {
                (*lo, *hi) = (lo.min(*x), hi.max(*x));
            }
        }
        if let Some(a) = self.accessors.get_mut(a) {
            set(a, "min", json!(lo));
            set(a, "max", json!(hi));
        }
        a
    }
    /// `doc` is the glTF document without buffers, bufferViews and accessors.
    pub fn finish(mut self, mut doc: Value) -> Vec<u8> {
        while !self.bin.len().is_multiple_of(4) {
            self.bin.push(0);
        }
        set(&mut doc, "asset", json!({"version": "2.0", "generator": "ttt_convert"}));
        set(&mut doc, "buffers", json!([{"byteLength": self.bin.len()}]));
        set(&mut doc, "bufferViews", json!(self.views));
        set(&mut doc, "accessors", json!(self.accessors));
        let mut js = doc.to_string().into_bytes();
        while !js.len().is_multiple_of(4) {
            js.push(b' ');
        }
        let mut out = Vec::with_capacity(28 + js.len() + self.bin.len());
        out.extend(b"glTF");
        out.extend(2u32.to_le_bytes());
        out.extend(((28 + js.len() + self.bin.len()) as u32).to_le_bytes());
        out.extend((js.len() as u32).to_le_bytes());
        out.extend(b"JSON");
        out.extend(js);
        out.extend((self.bin.len() as u32).to_le_bytes());
        out.extend(b"BIN\0");
        out.extend(self.bin);
        out
    }
}
