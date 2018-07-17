use jni::objects::JString;
use jni::sys::*;
use jni::JNIEnv;

#[inline]
pub fn convert_retval_i8(_env: &JNIEnv, input: i8) -> jbyte {
    input
}

#[inline]
pub fn convert_arg_jbyte(_env: &JNIEnv, input: jbyte) -> i8 {
    input
}

#[inline]
pub fn convert_retval_i32(_env: &JNIEnv, input: i32) -> jint {
    input
}

#[inline]
pub fn convert_arg_jint(_env: &JNIEnv, input: jint) -> i32 {
    input
}

#[inline]
pub fn convert_retval_u8(_env: &JNIEnv, input: u8) -> jboolean {
    input
}

#[inline]
pub fn convert_arg_jboolean(_env: &JNIEnv, input: jboolean) -> u8 {
    input
}

#[inline]
pub fn convert_retval_i16(_env: &JNIEnv, input: i16) -> jshort {
    input
}

#[inline]
pub fn convert_arg_jshort(_env: &JNIEnv, input: jshort) -> i16 {
    input
}

#[inline]
pub fn convert_retval_u16(_env: &JNIEnv, input: u16) -> jchar {
    input
}

#[inline]
pub fn convet_arg_jchar(_env: &JNIEnv, input: jchar) -> u16 {
    input
}

#[inline]
pub fn convert_retval_i64(_env: &JNIEnv, input: i64) -> jlong {
    input
}

#[inline]
pub fn convert_arg_jlong(_env: &JNIEnv, input: jlong) -> i64 {
    input
}

#[inline]
pub fn convert_retval_f32(_env: &JNIEnv, input: f32) -> jfloat {
    input
}

#[inline]
pub fn convert_arg_jfloat(_env: &JNIEnv, input: jfloat) -> f32 {
    input
}

#[inline]
pub fn convert_retval_f64(_env: &JNIEnv, input: f64) -> jdouble {
    input
}

#[inline]
pub fn convert_arg_jdouble(_env: &JNIEnv, input: jdouble) -> f64 {
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

/// Converts a string argument from java into a heap owned rust string.
#[inline]
pub fn convert_arg_jstring(env: &JNIEnv, input: JString) -> String {
    env.get_string(input)
        .expect("Could not get java string")
        .into()
}
