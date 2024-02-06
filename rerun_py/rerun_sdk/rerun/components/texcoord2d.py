# DO NOT EDIT! This file was auto-generated by crates/re_types_builder/src/codegen/python.rs
# Based on "crates/re_types/definitions/rerun/components/texcoord2d.fbs".

# You can extend this class by creating a "Texcoord2DExt" class in "texcoord2d_ext.py".

from __future__ import annotations

from .. import datatypes
from .._baseclasses import ComponentBatchMixin

__all__ = ["Texcoord2D", "Texcoord2DBatch", "Texcoord2DType"]


class Texcoord2D(datatypes.Vec2D):
    """
    **Component**: A 2D texture UV coordinate.

    Texture coordinates specify a position on a 2D texture.
    A range from 0-1 covers the entire texture in the respective dimension.
    Unless configured otherwise, the texture repeats outside of this range.
    Rerun uses top-left as the origin for UV coordinates.

      0     U     1
    0 + --------- →
      |           .
    V |           .
      |           .
    1 ↓ . . . . . .

    This is the same convention as in Vulkan/Metal/DX12/WebGPU, but (!) unlike OpenGL,
    which places the origin at the bottom-left.
    """

    # You can define your own __init__ function as a member of Texcoord2DExt in texcoord2d_ext.py

    # Note: there are no fields here because Texcoord2D delegates to datatypes.Vec2D
    pass


class Texcoord2DType(datatypes.Vec2DType):
    _TYPE_NAME: str = "rerun.components.Texcoord2D"


class Texcoord2DBatch(datatypes.Vec2DBatch, ComponentBatchMixin):
    _ARROW_TYPE = Texcoord2DType()