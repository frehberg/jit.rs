use function::UncompiledFunction;
use function::Abi::CDecl;
use types::get;
use std::os::raw::c_long;
use types::{consts, CowType, Type};
use util::from_ptr;
use value::Val;
use std::ffi::CStr;
use std::mem;
use raw::*;
/// A type that can be compiled into a LibJIT representation
///
/// The lifetime is the lifetime of the value
pub trait Compile<'a> {
    /// Get a JIT representation of this value
    fn compile(self, func:&'a UncompiledFunction) -> &'a Val;
    /// Get the type descriptor that represents this type
    fn get_type() -> CowType<'a>;
}
impl<'a> Compile<'a> for () {
    #[inline(always)]
    fn compile(self, func:&'a UncompiledFunction) -> &'a Val {
        Val::new(func, consts::get_void())
    }
    #[inline(always)]
    fn get_type() -> CowType<'a> {
        consts::get_void().into()
    }
}
compile_prims!{
    (f64, f64) => (get_float64, jit_value_create_float64_constant),
    (f32, f32) => (get_float32, jit_value_create_float32_constant),
    (isize, c_long) => (get_nint, jit_value_create_nint_constant),
    (usize, c_long) => (get_nuint, jit_value_create_nint_constant),
    (i64, c_long) => (get_long, jit_value_create_long_constant),
    (u64, c_long) => (get_ulong, jit_value_create_long_constant),
    (i32, c_long) => (get_int, jit_value_create_nint_constant),
    (u32, c_long) => (get_uint, jit_value_create_nint_constant),
    (i16, c_long) => (get_short, jit_value_create_nint_constant),
    (u16, c_long) => (get_ushort, jit_value_create_nint_constant),
    (i8, c_long) => (get_sbyte, jit_value_create_nint_constant),
    (u8, c_long) => (get_ubyte, jit_value_create_nint_constant),
    (bool, c_long) => (get_sys_bool, jit_value_create_nint_constant),
    (char, c_long) => (get_sys_char, jit_value_create_nint_constant)
}
impl<'a, T> Compile<'a> for *const T where T:Compile<'a> + Sized + 'a {
    #[inline(always)]
    fn compile(self, func:&'a UncompiledFunction) -> &'a Val {
        unsafe {
            let ty = Self::get_type();
            jit_value_create_nint_constant(
                func.into(),
                (&*ty).into(),
                mem::transmute(self)
            ).into()
        }
    }
    #[inline(always)]
    fn get_type() -> CowType<'a> {
        Type::new_pointer(&get::<T>()).into()
    }
}
impl<'a, T> Compile<'a> for *mut T where T:Compile<'a> + Sized + 'a {
    #[inline(always)]
    fn compile(self, func:&'a UncompiledFunction) -> &'a Val {
        unsafe {
            let ty = Self::get_type();
            jit_value_create_nint_constant(
                func.into(),
                (&*ty).into(),
                mem::transmute(self)
            ).into()
        }
    }
    #[inline(always)]
    fn get_type() -> CowType<'a> {
        Type::new_pointer(&get::<T>()).into()
    }
}
impl<'a, T> Compile<'a> for &'a mut T where T:Compile<'a> + Sized + 'a {
    #[inline(always)]
    fn compile(self, func:&'a UncompiledFunction) -> &'a Val {
        unsafe {
            let ty = Self::get_type();
            jit_value_create_nint_constant(
                func.into(),
                (&*ty).into(),
                mem::transmute(self)
            ).into()
        }
    }
    #[inline(always)]
    fn get_type() -> CowType<'a> {
        Type::new_pointer(&get::<T>()).into()
    }
}
impl<'a, T> Compile<'a> for &'a T where T:Compile<'a> + Sized + 'a {
    #[inline(always)]
    fn compile(self, func:&'a UncompiledFunction) -> &'a Val {
        unsafe {
            let ty = Self::get_type();
            jit_value_create_nint_constant(
                func.into(),
                (&*ty).into(),
                mem::transmute(self)
            ).into()
        }
    }
    #[inline(always)]
    fn get_type() -> CowType<'a> {
        Type::new_pointer(&get::<T>()).into()
    }
}
impl<'a> Compile<'a> for &'a CStr {
    #[inline(always)]
    fn compile(self, func:&'a UncompiledFunction) -> &'a Val {
        unsafe {
            let ty = Self::get_type();
            jit_value_create_nint_constant(
                func.into(),
                (&*ty).into(),
                mem::transmute(self.as_ptr())
            ).into()
        }
    }
    #[inline(always)]
    fn get_type() -> CowType<'a> {
        Type::new_pointer(consts::get_sys_char()).into()
    }
}

impl<'a> Compile<'a> for &'a str {
    #[inline(always)]
    fn compile(self, func:&'a UncompiledFunction) -> &'a Val {
        unsafe {
            let ty = Self::get_type();
            let structure = Val::new(func, &ty);
            func.insn_store_relative(structure, 0, func.insn_of(mem::transmute::<_, isize>(self.as_ptr())));
            func.insn_store_relative(structure, mem::size_of::<usize>(), func.insn_of(self.len()));
            structure
        }
    }
    #[inline(always)]
    fn get_type() -> CowType<'a> {
        let ty = Type::new_struct(&mut [&get::<*const u8>(), &get::<usize>()]);
        unsafe {
            jit_type_set_size_and_alignment((&ty).into(), mem::size_of::<*mut str>() as i64, mem::align_of::<*mut str>() as i64);
        }
        ty.into()
    }
}
impl<'a, T> Compile<'a> for (T, ) where T: Compile<'a> {
    #[inline(always)]
    fn compile(self, func:&'a UncompiledFunction) -> &'a Val {
        self.0.compile(func)
    }
    #[inline(always)]
    fn get_type() -> CowType<'a> {
        T::get_type()
    }
}
compile_tuple!(A, B => a, b);
compile_tuple!(A, B, C => a, b, c);
compile_tuple!(A, B, C, D => a, b, c, d);
compile_tuple!(A, B, C, D, E => a, b, c, d, e);
compile_func!(fn() -> R, fn() -> R, extern fn() -> R);
compile_func!(fn(A) -> R, fn(A) -> R, extern fn(A) -> R);
compile_func!(fn(A, B) -> R, fn(A, B) -> R, extern fn(A, B) -> R);
compile_func!(fn(A, B, C) -> R, fn(A, B, C) -> R, extern fn(A, B, C) -> R);
compile_func!(fn(A, B, C, D) -> R, fn(A, B, C, D) -> R, extern fn(A, B, C, D) -> R);
