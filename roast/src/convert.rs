use jni::sys::*;
use jni::JNIEnv;

#[inline]
pub fn convert_retval_i8(_env: &JNIEnv, input: i8) -> jbyte {
    input
}

#[inline]
pub fn convert_retval_i32(_env: &JNIEnv, input: i32) -> jint {
    input
}

#[inline]
pub fn convert_retval_u8(_env: &JNIEnv, input: u8) -> jboolean {
    input
}

#[inline]
pub fn convert_retval_i16(_env: &JNIEnv, input: i16) -> jshort {
    input
}

#[inline]
pub fn convert_retval_u16(_env: &JNIEnv, input: u16) -> jchar {
    input
}

#[inline]
pub fn convert_retval_u32(_env: &JNIEnv, input: u32) -> jlong {
    input.into()
}

#[inline]
pub fn convert_retval_i64(_env: &JNIEnv, input: i64) -> jlong {
    input
}

#[inline]
pub fn convert_retval_f32(_env: &JNIEnv, input: f32) -> jfloat {
    input
}

#[inline]
pub fn convert_retval_f64(_env: &JNIEnv, input: f64) -> jdouble {
    input
}

/// Converts a return value rust string into a java string.
///
/// Note that right now we panic if the string can't be created,
/// but I'm not sure if this is the right approach since it's
/// non-recoverable.
#[inline]
pub fn convert_retval_string(env: &JNIEnv, input: String) -> jstring {
    env.new_string(input)
        .expect("Could not create Java String for return value!")
        .into_inner()
}
