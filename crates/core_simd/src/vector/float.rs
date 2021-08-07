#![allow(non_camel_case_types)]

use crate::{LaneCount, SupportedLaneCount};

/// Implements inherent methods for a float vector `$name` containing multiple
/// `$lanes` of float `$type`, which uses `$bits_ty` as its binary
/// representation. Called from `define_float_vector!`.
macro_rules! impl_float_vector {
    { $name:ident, $type:ident, $bits_ty:ident, $mask_ty:ident, $mask_impl_ty:ident } => {
        impl<const LANES: usize> $name<LANES>
        where
            LaneCount<LANES>: SupportedLaneCount,
        {
            /// Raw transmutation to an unsigned integer vector type with the
            /// same size and number of lanes.
            #[inline]
            pub fn to_bits(self) -> crate::$bits_ty<LANES> {
                assert_eq!(core::mem::size_of::<Self>(), core::mem::size_of::<crate::$bits_ty<LANES>>());
                unsafe { core::mem::transmute_copy(&self) }
            }

            /// Raw transmutation from an unsigned integer vector type with the
            /// same size and number of lanes.
            #[inline]
            pub fn from_bits(bits: crate::$bits_ty<LANES>) -> Self {
                assert_eq!(core::mem::size_of::<Self>(), core::mem::size_of::<crate::$bits_ty<LANES>>());
                unsafe { core::mem::transmute_copy(&bits) }
            }

            /// Produces a vector where every lane has the absolute value of the
            /// equivalently-indexed lane in `self`.
            #[inline]
            pub fn abs(self) -> Self {
                unsafe { crate::intrinsics::simd_fabs(self) }
            }

            /// Fused multiply-add.  Computes `(self * a) + b` with only one rounding error,
            /// yielding a more accurate result than an unfused multiply-add.
            ///
            /// Using `mul_add` *may* be more performant than an unfused multiply-add if the target
            /// architecture has a dedicated `fma` CPU instruction.  However, this is not always
            /// true, and will be heavily dependent on designing algorithms with specific target
            /// hardware in mind.
            #[inline]
            pub fn mul_add(self, a: Self, b: Self) -> Self {
                unsafe { crate::intrinsics::simd_fma(self, a, b) }
            }

            /// Produces a vector where every lane has the square root value
            /// of the equivalently-indexed lane in `self`
            #[inline]
            #[cfg(feature = "std")]
            pub fn sqrt(self) -> Self {
                unsafe { crate::intrinsics::simd_fsqrt(self) }
            }

            /// Takes the reciprocal (inverse) of each lane, `1/x`.
            #[inline]
            pub fn recip(self) -> Self {
                Self::splat(1.0) / self
            }

            /// Converts each lane from radians to degrees.
            #[inline]
            pub fn to_degrees(self) -> Self {
                // to_degrees uses a special constant for better precision, so extract that constant
                self * Self::splat($type::to_degrees(1.))
            }

            /// Converts each lane from degrees to radians.
            #[inline]
            pub fn to_radians(self) -> Self {
                self * Self::splat($type::to_radians(1.))
            }

            /// Returns true for each lane if it has a positive sign, including
            /// `+0.0`, `NaN`s with positive sign bit and positive infinity.
            #[inline]
            pub fn is_sign_positive(self) -> crate::$mask_ty<LANES> {
                !self.is_sign_negative()
            }

            /// Returns true for each lane if it has a negative sign, including
            /// `-0.0`, `NaN`s with negative sign bit and negative infinity.
            #[inline]
            pub fn is_sign_negative(self) -> crate::$mask_ty<LANES> {
                let sign_bits = self.to_bits() & crate::$bits_ty::splat((!0 >> 1) + 1);
                sign_bits.lanes_gt(crate::$bits_ty::splat(0))
            }

            /// Returns true for each lane if its value is `NaN`.
            #[inline]
            pub fn is_nan(self) -> crate::$mask_ty<LANES> {
                self.lanes_ne(self)
            }

            /// Returns true for each lane if its value is positive infinity or negative infinity.
            #[inline]
            pub fn is_infinite(self) -> crate::$mask_ty<LANES> {
                self.abs().lanes_eq(Self::splat(<$type>::INFINITY))
            }

            /// Returns true for each lane if its value is neither infinite nor `NaN`.
            #[inline]
            pub fn is_finite(self) -> crate::$mask_ty<LANES> {
                self.abs().lanes_lt(Self::splat(<$type>::INFINITY))
            }

            /// Returns true for each lane if its value is subnormal.
            #[inline]
            pub fn is_subnormal(self) -> crate::$mask_ty<LANES> {
                self.abs().lanes_ne(Self::splat(0.0)) & (self.to_bits() & Self::splat(<$type>::INFINITY).to_bits()).lanes_eq(crate::$bits_ty::splat(0))
            }

            /// Returns true for each lane if its value is neither neither zero, infinite,
            /// subnormal, or `NaN`.
            #[inline]
            pub fn is_normal(self) -> crate::$mask_ty<LANES> {
                !(self.abs().lanes_eq(Self::splat(0.0)) | self.is_nan() | self.is_subnormal() | self.is_infinite())
            }

            /// Replaces each lane with a number that represents its sign.
            ///
            /// * `1.0` if the number is positive, `+0.0`, or `INFINITY`
            /// * `-1.0` if the number is negative, `-0.0`, or `NEG_INFINITY`
            /// * `NAN` if the number is `NAN`
            #[inline]
            pub fn signum(self) -> Self {
                self.is_nan().select(Self::splat($type::NAN), Self::splat(1.0).copysign(self))
            }

            /// Returns each lane with the magnitude of `self` and the sign of `sign`.
            ///
            /// If any lane is a `NAN`, then a `NAN` with the sign of `sign` is returned.
            #[inline]
            pub fn copysign(self, sign: Self) -> Self {
                let sign_bit = sign.to_bits() & Self::splat(-0.).to_bits();
                let magnitude = self.to_bits() & !Self::splat(-0.).to_bits();
                Self::from_bits(sign_bit | magnitude)
            }

            /// Returns the minimum of each lane.
            ///
            /// If one of the values is `NAN`, then the other value is returned.
            #[inline]
            pub fn min(self, other: Self) -> Self {
                // TODO consider using an intrinsic
                self.is_nan().select(
                    other,
                    self.lanes_ge(other).select(other, self)
                )
            }

            /// Returns the maximum of each lane.
            ///
            /// If one of the values is `NAN`, then the other value is returned.
            #[inline]
            pub fn max(self, other: Self) -> Self {
                // TODO consider using an intrinsic
                self.is_nan().select(
                    other,
                    self.lanes_le(other).select(other, self)
                )
            }

            /// Restrict each lane to a certain interval unless it is NaN.
            ///
            /// For each lane in `self`, returns the corresponding lane in `max` if the lane is
            /// greater than `max`, and the corresponding lane in `min` if the lane is less
            /// than `min`.  Otherwise returns the lane in `self`.
            #[inline]
            pub fn clamp(self, min: Self, max: Self) -> Self {
                assert!(
                    min.lanes_le(max).all(),
                    "each lane in `min` must be less than or equal to the corresponding lane in `max`",
                );
                let mut x = self;
                x = x.lanes_lt(min).select(min, x);
                x = x.lanes_gt(max).select(max, x);
                x
            }
        }
    };
}

/// A SIMD vector of containing `LANES` `f32` values.
pub type SimdF32<const LANES: usize> = crate::Simd<f32, LANES>;

/// A SIMD vector of containing `LANES` `f64` values.
pub type SimdF64<const LANES: usize> = crate::Simd<f64, LANES>;

impl_float_vector! { SimdF32, f32, SimdU32, Mask32, SimdI32 }
impl_float_vector! { SimdF64, f64, SimdU64, Mask64, SimdI64 }

/// Vector of two `f32` values
pub type f32x2 = SimdF32<2>;

/// Vector of four `f32` values
pub type f32x4 = SimdF32<4>;

/// Vector of eight `f32` values
pub type f32x8 = SimdF32<8>;

/// Vector of 16 `f32` values
pub type f32x16 = SimdF32<16>;

/// Vector of two `f64` values
pub type f64x2 = SimdF64<2>;

/// Vector of four `f64` values
pub type f64x4 = SimdF64<4>;

/// Vector of eight `f64` values
pub type f64x8 = SimdF64<8>;
