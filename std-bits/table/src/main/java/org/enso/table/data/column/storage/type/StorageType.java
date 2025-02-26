package org.enso.table.data.column.storage.type;

import org.enso.base.polyglot.NumericConverter;

import java.time.LocalDate;
import java.time.LocalDateTime;
import java.time.LocalTime;
import java.time.ZonedDateTime;

/**
 * Represents an underlying internal storage type that can be mapped to the Value Type that is exposed to users.
 */
public sealed interface StorageType permits AnyObjectType, BooleanType, DateType, DateTimeType, FloatType, IntegerType, TextType, TimeOfDayType {
  /**
   * @return the StorageType that represents a given boxed item.
   */
  static StorageType forBoxedItem(Object item) {
    if (NumericConverter.isCoercibleToLong(item)) {
      return IntegerType.INT_64;
    }

    if (NumericConverter.isDecimalLike(item)) {
      return FloatType.FLOAT_64;
    }

    return switch (item) {
      case Boolean b -> BooleanType.INSTANCE;
      case LocalDate d -> DateType.INSTANCE;
      case LocalTime t -> TimeOfDayType.INSTANCE;
      case LocalDateTime d -> DateTimeType.INSTANCE;
      case ZonedDateTime d -> DateTimeType.INSTANCE;
      case String s -> TextType.VARIABLE_LENGTH;
      default -> null;
    };
  }
}
