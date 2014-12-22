use raw::*;
use compile::Compile;
use function::ABI;
use libc::{c_uint, c_void};
use std::fmt::{Show, Formatter, Result};
use std::kinds::marker::{ContravariantLifetime, NoCopy};
use std::mem;
use std::c_str::ToCStr;
use util::{NativeRef, mod};
/// The integer representation of a type
pub mod kind {
    use libc::c_int;
    bitflags!(
        flags TypeKind: c_int {
            const Void = 0,
            const SByte = 1,
            const UByte = 2,
            const Short = 3,
            const UShort = 4,
            const Int = 5,
            const UInt = 6,
            const NInt = 7,
            const NUInt = 8,
            const Long = 9,
            const ULong = 10,
            const Float32 = 11,
            const Float64 = 12,
            const NFloat = 13,
            const Struct = 14,
            const Union = 15,
            const Signature = 16,
            const Pointer = 17,
            const FirstTagged = 2,
            const SysBool = 10009,
            const SysChar = 10010
        }
    );
}
impl Show for Type {
    fn fmt(&self, fmt: &mut Formatter) -> Result {
        try!(util::dump(|fd| {
            unsafe { jit_dump_type(mem::transmute(fd), self.as_ptr()) };
        })).fmt(fmt)
    }
}
/// A single field of a struct
pub struct Field<'a> {
    /// The index of the field
    pub index: c_uint,
    _type: jit_type_t,
    marker: ContravariantLifetime<'a>
}
impl<'a> PartialEq for Field<'a> {
    fn eq(&self, other:&Field<'a>) -> bool {
        self.index == other.index && self._type == other._type
    }
}
impl<'a> Field<'a> {
    #[inline]
    /// Get the field's name or none if it lacks one
    pub fn get_name(&self) -> Option<String> {
        unsafe {
            let c_name = jit_type_get_name(self._type, self.index);
            if c_name.is_null() {
                None
            } else {
                Some(String::from_raw_buf(mem::transmute(c_name)))
            }
        }
    }
    #[inline(always)]
    /// Get the type of the field
    pub fn get_type(&self) -> Type {
        unsafe {
            NativeRef::from_ptr(jit_type_get_field(self._type, self.index))
        }
    }
    #[inline(always)]
    /// Get the offset of the field
    pub fn get_offset(&self) -> uint {
        unsafe {
            jit_type_get_offset(self._type, self.index) as uint
        }
    }
}
/// Iterates through all the fields of a struct
pub struct Fields<'a> {
    _type: jit_type_t,
    index: c_uint,
    length: c_uint,
    marker: ContravariantLifetime<'a>
}
impl<'a> Fields<'a> {
    #[inline(always)]
    fn new(ty:Type) -> Fields<'a> {
        unsafe {
            Fields {
                _type: ty.as_ptr(),
                index: 0 as c_uint,
                length: jit_type_num_fields(ty.as_ptr()),
                marker: ContravariantLifetime::<'a>
            }
        }
    }
}
impl<'a> Iterator<Field<'a>> for Fields<'a> {
    fn next(&mut self) -> Option<Field<'a>> {
        if self.index < self.length {
            let index = self.index;
            self.index += 1;
            Some(Field {
                index: index,
                _type: self._type,
                marker: ContravariantLifetime::<'a>
            })
        } else {
            None
        }
    }
    fn size_hint(&self) -> (uint, Option<uint>) {
        ((self.length - self.index) as uint, None)
    }
}
/// Iterator through all the arguments a function takes
pub struct Params<'a> {
    _type: jit_type_t,
    index: c_uint,
    length: c_uint,
    marker: ContravariantLifetime<'a>
}
impl<'a> Params<'a> {
    fn new(ty:Type) -> Params<'a> {
        unsafe {
            Params {
                _type: ty.as_ptr(),
                index: 0,
                length: jit_type_num_params(ty.as_ptr()),
                marker: ContravariantLifetime::<'a>
            }
        }
    }
}
impl<'a> Iterator<Type> for Params<'a> {
    fn next(&mut self) -> Option<Type> {
        if self.index < self.length {
            let index = self.index;
            self.index += 1;
            unsafe { NativeRef::from_opt_ptr(jit_type_get_param(self._type, index)) }
        } else {
            None
        }
    }
    #[inline]
    fn size_hint(&self) -> (uint, Option<uint>) {
        ((self.length - self.index) as uint, None)
    }
}
/// An object that represents a native system type.
/// Each `Type` represents a basic system type, be it a primitive, a struct, a
/// union, a pointer, or a function signature. The library uses this information
/// to lay out values in memory.
/// Types are not attached to a context so they are reference-counted by LibJIT,
/// so internally they are represented as `Rc<TypeData>`.
pub struct Type {
    _type: jit_type_t,
    no_copy: NoCopy
}
impl NativeRef for Type {
    #[inline(always)]
    unsafe fn as_ptr(&self) -> jit_type_t {
        self._type
    }
    #[inline(always)]
    unsafe fn from_ptr(ptr:jit_type_t) -> Type {
        Type {
            _type: ptr,
            no_copy: NoCopy
        }
    }
}
impl PartialEq for Type {
    fn eq(&self, other: &Type) -> bool {
        self._type == other._type
    }
}
impl Clone for Type {
    #[inline]
    /// Make a copy of the type descriptor by increasing its reference count.
    fn clone(&self) -> Type {
        unsafe {
            NativeRef::from_ptr(jit_type_copy(self.as_ptr()))
        }
    }
}
#[unsafe_destructor]
impl Drop for Type {
    #[inline(always)]
    /// Free a type descriptor by decreasing its reference count.
    /// This function is safe to use on pre-defined types, which are never
    /// actually freed.
    fn drop(&mut self) {
        unsafe {
            jit_type_free(self.as_ptr());
        }
    }
}
extern fn free_data<T:'static>(data: *mut c_void) {
    unsafe {
        let actual_data:Box<T> = mem::transmute(data);
        mem::drop(actual_data);
    }
}
impl Type {
    /// Create a type descriptor for a function signature.
    pub fn create_signature(abi: ABI, return_type: Type, params: &mut [Type]) -> Type {
        unsafe {
            let mut native_params:Vec<jit_type_t> = params.iter().map(|param| param.as_ptr()).collect();
            let signature = jit_type_create_signature(abi as jit_abi_t, return_type.as_ptr(), native_params.as_mut_ptr(), params.len() as c_uint, 1);
            NativeRef::from_ptr(signature)
        }
    }
    #[inline(always)]
    /// Create a type descriptor for a structure.
    pub fn create_struct(fields: &mut [Type]) -> Type {
        unsafe {
            let mut native_fields:Vec<_> = fields.iter().map(|field| field.as_ptr()).collect();
            NativeRef::from_ptr(jit_type_create_struct(native_fields.as_mut_ptr(), fields.len() as c_uint, 1))
        }
    }
    #[inline(always)]
    /// Create a type descriptor for a union.
    pub fn create_union(fields: &mut [Type]) -> Type {
        unsafe {
            let mut native_fields:Vec<_> = fields.iter().map(|field| field.as_ptr()).collect();
            NativeRef::from_ptr(jit_type_create_union(native_fields.as_mut_ptr(), fields.len() as c_uint, 1))
        }
    }
    #[inline(always)]
    /// Create a type descriptor for a pointer to another type.
    pub fn create_pointer(pointee: Type) -> Type {
        unsafe {
            let ptr = jit_type_create_pointer(pointee.as_ptr(), 1);
            NativeRef::from_ptr(ptr)
        }
    }
    #[inline(always)]
    /// Create a new tagged type
    pub fn create_tagged<T:'static>(ty:Type, kind: kind::TypeKind, data: Box<T>) -> Type {
        unsafe {
            let ty = jit_type_create_tagged(ty.as_ptr(), kind.bits(), mem::transmute(&*data), Some(free_data::<T>), 1);
            mem::forget(data);
            NativeRef::from_ptr(ty)
        }
    }
    #[inline(always)]
    /// Get the size of this type in bytes.
    pub fn get_size(&self) -> uint {
        unsafe {
            jit_type_get_size(self.as_ptr()) as uint
        }
    }
    #[inline(always)]
    /// Get the alignment of this type in bytes.
    pub fn get_alignment(&self) -> uint {
        unsafe {
            jit_type_get_alignment(self.as_ptr()) as uint
        }
    }
    #[inline]
    /// Get a value that indicates the kind of this type. This allows callers to
    /// quickly classify a type to determine how it should be handled further.
    pub fn get_kind(&self) -> kind::TypeKind {
        unsafe {
            mem::transmute(jit_type_get_kind(self.as_ptr()))
        }
    }
    #[inline(always)]
    /// Get the type that is referred to by this pointer type.
    pub fn get_ref(&self) -> Option<Type> {
        unsafe {
            NativeRef::from_ptr(jit_type_get_ref(self.as_ptr()))
        }
    }

    #[inline(always)]
    pub fn get_tagged_data<T:'static>(&self) -> Option<&T> {
        unsafe {
            mem::transmute(jit_type_get_tagged_data(self.as_ptr()))
        }
    }
    #[inline(always)]
    pub fn set_tagged_data<T:'static>(&self, data: Box<T>) {
        unsafe {
            jit_type_set_tagged_data(self.as_ptr(), mem::transmute(&*data), Some(free_data::<T>));
            mem::forget(data);
        }
    }
    #[inline(always)]
    /// Get the type returned by this function type.
    pub fn get_return(&self) -> Option<Type> {
        unsafe {
            NativeRef::from_ptr(jit_type_get_return(self.as_ptr()))
        }
    }
    /// Set the field or parameter names of this type.
    pub fn set_names<T:ToCStr>(&self, names:&[T]) -> bool {
        unsafe {
            let native_names : Vec<*const i8> = names.iter().map(|name| name.to_c_str().into_inner()).collect();
            jit_type_set_names(self.as_ptr(), native_names.as_ptr() as *mut *mut i8, names.len() as u32) != 0
        }
    }
    #[inline(always)]
    /// Iterator over the type's fields
    pub fn fields<'a>(&'a self) -> Fields<'a> {
        Fields::new(self.clone())
    }
    #[inline(always)]
    /// Iterator over the function signature's parameters
    pub fn params<'a>(&'a self) -> Params<'a> {
        Params::new(self.clone())
    }
    #[inline]
    /// Find the field/parameter index for a particular name.
    pub fn find_name<'b, T:ToCStr>(&'b self, name:T) -> Field<'b> {
        name.with_c_str(|c_name| unsafe {
            Field {
                index: jit_type_find_name(self.as_ptr(), c_name),
                _type: self.as_ptr(),
                marker: ContravariantLifetime::<'b>
            }
        })
    }
    #[inline(always)]
    /// Check if this is a pointer
    pub fn is_pointer(&self) -> bool {
        unsafe {
            jit_type_is_pointer(self.as_ptr()) != 0
        }
    }
}
#[inline(always)]
/// Get the Rust type given as a type descriptor
pub fn get<T: Compile>() -> Type {
    Compile::jit_type(None::<T>)
}