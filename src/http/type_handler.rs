use crate::TypeSystemError;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

/// This macro configures casts of a single entity type.
#[macro_export]
macro_rules! configure_type_system {
    ($ts:expr, $base:ty) => {
        $ts.put_cast(|src: &$base| { src });
        $ts.put_cast_mut(|src: &mut $base| { src });
    };
    ($ts:expr, $base:ty, $t1:tt) => {
        $ts.put_cast(|src: &$base| { src as &dyn $t1});
        $ts.put_cast_mut(|src: &mut $base| { src as &mut dyn ($t1) });
        configure_type_system!($ts, $base);
    };
    ($ts:expr, $base:ty, $t1:tt, $($t2:tt),*) => {
        $ts.put_cast(|src: &$base| { src as &dyn ($t1) });
        $ts.put_cast_mut(|src: &mut $base| { src as &mut dyn ($t1) });
        configure_type_system!($ts, $base, $($t2),+);
    };
}

/// Builder for the type system
#[derive(Debug, Default)]
pub struct TypeSystemBuilder {
  types: HashMap<TypeId, HashMap<TypeId, TypeCasterWrapper>>,
  types_mut: HashMap<TypeId, HashMap<TypeId, TypeCasterWrapperMut>>,
}

/// Holds an immutable type system containing casting information
#[derive(Debug, Clone)]
pub struct TypeSystem(Arc<TypeSystemBuilder>);
impl TypeSystem {
  pub(crate) fn type_cast_wrapper(
    &self,
    src: TypeId,
    target: TypeId,
  ) -> Result<TypeCasterWrapper, TypeSystemError> {
    let Some(type_map) = self.0.types.get(&src) else {
      return Err(TypeSystemError::SourceTypeUnknown);
    };

    type_map.get(&target).cloned().ok_or(TypeSystemError::NoCastToTargetType)
  }

  pub(crate) fn type_cast_wrapper_mut(
    &self,
    src: TypeId,
    target: TypeId,
  ) -> Result<TypeCasterWrapperMut, TypeSystemError> {
    let Some(type_map) = self.0.types_mut.get(&src) else {
      return Err(TypeSystemError::SourceTypeUnknown);
    };

    type_map.get(&target).cloned().ok_or(TypeSystemError::NoCastToTargetType)
  }
}

impl TypeSystemBuilder {
  /// Add a cast to the type system. Use the macro configure_type_system! to call this fn.
  /// Calling this fn directly is just unneeded boilerplate.
  pub fn put_cast<SRC: Any + 'static, DST: Any + ?Sized + 'static>(
    &mut self,
    mapper: impl Fn(&SRC) -> &DST + Send + Sync + 'static,
  ) {
    let caster = Arc::new(move |input: &dyn Any, down: Box<dyn Any>| {
      let Some(input) = input.downcast_ref::<SRC>() else { crate::util::unreachable() };

      let Ok(mut downstream) = down.downcast::<DownstreamWrapper<DST>>() else {
        crate::util::unreachable()
      };
      let Some(downstream_fn) = downstream.0.take() else { crate::util::unreachable() };
      downstream_fn(mapper(input))
    });

    let wrapper =
      TypeCasterWrapper { src: TypeId::of::<SRC>(), dst: TypeId::of::<DST>(), handler: caster };

    let type_map = self.types.entry(TypeId::of::<SRC>()).or_default();
    type_map.insert(TypeId::of::<DST>(), wrapper);
  }

  /// Add a mutable cast to the type system. Use the macro configure_type_system! to call this fn.
  /// Calling this fn directly is just unneeded boilerplate.
  pub fn put_cast_mut<SRC: Any + 'static, DST: Any + ?Sized + 'static>(
    &mut self,
    mapper: impl Fn(&mut SRC) -> &mut DST + Send + Sync + 'static,
  ) {
    let caster = Arc::new(move |input: &mut dyn Any, down: Box<dyn Any>| {
      let Some(input) = input.downcast_mut::<SRC>() else { crate::util::unreachable() };

      let Ok(mut downstream) = down.downcast::<DownstreamWrapperMut<DST>>() else {
        crate::util::unreachable()
      };
      let Some(downstream_fn) = downstream.0.take() else { crate::util::unreachable() };
      downstream_fn(mapper(input))
    });

    let wrapper =
      TypeCasterWrapperMut { src: TypeId::of::<SRC>(), dst: TypeId::of::<DST>(), handler: caster };

    let type_map = self.types_mut.entry(TypeId::of::<SRC>()).or_default();
    type_map.insert(TypeId::of::<DST>(), wrapper);
  }

  /// Builds the type system and makes it immutable
  pub fn build(self) -> TypeSystem {
    TypeSystem(Arc::new(self))
  }
}

#[allow(clippy::type_complexity)] //Aliasing the handler type would just cause confusion
#[derive(Clone)]
pub(crate) struct TypeCasterWrapper {
  src: TypeId,
  dst: TypeId,
  handler: Arc<dyn Fn(&dyn Any, Box<dyn Any>) -> Box<dyn Any> + Send + Sync>,
}

impl TypeCasterWrapper {
  pub(crate) fn call<DST: Any + 'static + ?Sized, RET: Any + 'static>(
    &self,
    src: &dyn Any,
    receiver: impl FnOnce(&DST) -> RET + 'static,
  ) -> Result<RET, TypeSystemError> {
    if src.type_id() != self.src {
      return Err(TypeSystemError::SourceTypeDoesNotMatch);
    }

    if TypeId::of::<DST>() != self.dst {
      return Err(TypeSystemError::NoCastToTargetType);
    }

    let downstream_wrapper = Box::new(DownstreamWrapper::<DST>(Some(Box::new(move |dst| {
      Box::new(receiver(dst)) as Box<dyn Any>
    })))) as Box<dyn Any>;
    let Ok(result) = (self.handler)(src, downstream_wrapper).downcast::<RET>() else {
      crate::util::unreachable()
    };
    Ok(*result)
  }
}
impl Debug for TypeCasterWrapper {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_str("TypeCasterWrapper")
  }
}

#[allow(clippy::type_complexity)] //Aliasing the handler type would just cause confusion
#[derive(Clone)]
pub(crate) struct TypeCasterWrapperMut {
  src: TypeId,
  dst: TypeId,
  handler: Arc<dyn Fn(&mut dyn Any, Box<dyn Any>) -> Box<dyn Any> + Send + Sync>,
}

impl TypeCasterWrapperMut {
  pub(crate) fn call<DST: Any + 'static + ?Sized, RET: Any + 'static>(
    &self,
    src: &mut dyn Any,
    receiver: impl FnOnce(&mut DST) -> RET + 'static,
  ) -> Result<RET, TypeSystemError> {
    if Any::type_id(src) != self.src {
      return Err(TypeSystemError::SourceTypeDoesNotMatch);
    }

    if TypeId::of::<DST>() != self.dst {
      return Err(TypeSystemError::NoCastToTargetType);
    }

    let downstream_wrapper = Box::new(DownstreamWrapperMut::<DST>(Some(Box::new(move |dst| {
      Box::new(receiver(dst)) as Box<dyn Any>
    })))) as Box<dyn Any>;
    let Ok(result) = (self.handler)(src, downstream_wrapper).downcast::<RET>() else {
      crate::util::unreachable()
    };
    Ok(*result)
  }
}

impl Debug for TypeCasterWrapperMut {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_str("TypeCasterWrapper")
  }
}

#[allow(clippy::type_complexity)] //This is already just an alias. I am not aliasing it again.
struct DownstreamWrapper<T: ?Sized>(Option<Box<dyn FnOnce(&T) -> Box<dyn Any>>>);

#[allow(clippy::type_complexity)] //This is already just an alias. I am not aliasing it again.
struct DownstreamWrapperMut<T: ?Sized>(Option<Box<dyn FnOnce(&mut T) -> Box<dyn Any>>>);
