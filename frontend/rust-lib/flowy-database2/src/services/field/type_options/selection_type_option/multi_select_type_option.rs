use crate::entities::{FieldType, SelectOptionCellDataPB, SelectOptionFilterPB};
use crate::services::cell::{CellDataChangeset, FromCellString, TypeCellData};
use collab::core::lib0_any_ext::Lib0AnyMapExtension;
use std::cmp::{min, Ordering};

use crate::services::field::{
  default_order, SelectOption, SelectOptionCellChangeset, SelectOptionIds,
  SelectTypeOptionSharedAction, SelectedSelectOptions, TypeOption, TypeOptionCellData,
  TypeOptionCellDataCompare, TypeOptionCellDataFilter,
};
use collab_database::fields::{TypeOptionData, TypeOptionDataBuilder};
use flowy_error::FlowyResult;
use serde::{Deserialize, Serialize};

// Multiple select
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MultiSelectTypeOption {
  pub options: Vec<SelectOption>,
  pub disable_color: bool,
}

impl TypeOption for MultiSelectTypeOption {
  type CellData = SelectOptionIds;
  type CellChangeset = SelectOptionCellChangeset;
  type CellProtobufType = SelectOptionCellDataPB;
  type CellFilter = SelectOptionFilterPB;
}

impl From<TypeOptionData> for MultiSelectTypeOption {
  fn from(data: TypeOptionData) -> Self {
    data
      .get_str_value("content")
      .map(|s| serde_json::from_str::<MultiSelectTypeOption>(&s).unwrap_or_default())
      .unwrap_or_default()
  }
}

impl From<MultiSelectTypeOption> for TypeOptionData {
  fn from(data: MultiSelectTypeOption) -> Self {
    let content = serde_json::to_string(&data).unwrap_or_default();
    TypeOptionDataBuilder::new()
      .insert("content", content)
      .build()
  }
}

impl TypeOptionCellData for MultiSelectTypeOption {
  fn convert_to_protobuf(
    &self,
    cell_data: <Self as TypeOption>::CellData,
  ) -> <Self as TypeOption>::CellProtobufType {
    self.get_selected_options(cell_data).into()
  }

  fn decode_type_option_cell_str(
    &self,
    cell_str: String,
  ) -> FlowyResult<<Self as TypeOption>::CellData> {
    SelectOptionIds::from_cell_str(&cell_str)
  }
}

impl SelectTypeOptionSharedAction for MultiSelectTypeOption {
  fn number_of_max_options(&self) -> Option<usize> {
    None
  }

  fn options(&self) -> &Vec<SelectOption> {
    &self.options
  }

  fn mut_options(&mut self) -> &mut Vec<SelectOption> {
    &mut self.options
  }
}

impl CellDataChangeset for MultiSelectTypeOption {
  fn apply_changeset(
    &self,
    changeset: <Self as TypeOption>::CellChangeset,
    type_cell_data: Option<TypeCellData>,
  ) -> FlowyResult<(String, <Self as TypeOption>::CellData)> {
    let insert_option_ids = changeset
      .insert_option_ids
      .into_iter()
      .filter(|insert_option_id| {
        self
          .options
          .iter()
          .any(|option| &option.id == insert_option_id)
      })
      .collect::<Vec<String>>();

    let select_option_ids = match type_cell_data {
      None => SelectOptionIds::from(insert_option_ids),
      Some(type_cell_data) => {
        let mut select_ids: SelectOptionIds = type_cell_data.cell_str.into();
        for insert_option_id in insert_option_ids {
          if !select_ids.contains(&insert_option_id) {
            select_ids.push(insert_option_id);
          }
        }

        for delete_option_id in changeset.delete_option_ids {
          select_ids.retain(|id| id != &delete_option_id);
        }

        tracing::trace!("Multi-select cell data: {}", select_ids.to_string());
        select_ids
      },
    };
    Ok((select_option_ids.to_string(), select_option_ids))
  }
}

impl TypeOptionCellDataFilter for MultiSelectTypeOption {
  fn apply_filter(
    &self,
    filter: &<Self as TypeOption>::CellFilter,
    field_type: &FieldType,
    cell_data: &<Self as TypeOption>::CellData,
  ) -> bool {
    if !field_type.is_multi_select() {
      return true;
    }
    let selected_options =
      SelectedSelectOptions::from(self.get_selected_options(cell_data.clone()));
    filter.is_visible(&selected_options, FieldType::MultiSelect)
  }
}

impl TypeOptionCellDataCompare for MultiSelectTypeOption {
  fn apply_cmp(
    &self,
    cell_data: &<Self as TypeOption>::CellData,
    other_cell_data: &<Self as TypeOption>::CellData,
  ) -> Ordering {
    for i in 0..min(cell_data.len(), other_cell_data.len()) {
      let order = match (
        cell_data
          .get(i)
          .and_then(|id| self.options.iter().find(|option| &option.id == id)),
        other_cell_data
          .get(i)
          .and_then(|id| self.options.iter().find(|option| &option.id == id)),
      ) {
        (Some(left), Some(right)) => left.name.cmp(&right.name),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => default_order(),
      };

      if order.is_ne() {
        return order;
      }
    }
    default_order()
  }
}

#[cfg(test)]
mod tests {
  use crate::entities::FieldType;
  use crate::services::cell::CellDataChangeset;
  use crate::services::field::type_options::selection_type_option::*;
  use crate::services::field::MultiSelectTypeOption;
  use crate::services::field::{CheckboxTypeOption, TypeOptionTransform};

  #[test]
  fn multi_select_transform_with_checkbox_type_option_test() {
    let checkbox_type_option = CheckboxTypeOption { is_selected: false };
    let mut multi_select = MultiSelectTypeOption::default();
    multi_select.transform_type_option(FieldType::Checkbox, checkbox_type_option.clone().into());
    debug_assert_eq!(multi_select.options.len(), 2);

    // Already contain the yes/no option. It doesn't need to insert new options
    multi_select.transform_type_option(FieldType::Checkbox, checkbox_type_option.into());
    debug_assert_eq!(multi_select.options.len(), 2);
  }

  #[test]
  fn multi_select_transform_with_single_select_type_option_test() {
    let google = SelectOption::new("Google");
    let facebook = SelectOption::new("Facebook");
    let mut single_select = SingleSelectTypeOption {
      options: vec![google, facebook],
      disable_color: false,
    };
    let mut multi_select = MultiSelectTypeOption {
      options: vec![],
      disable_color: false,
    };
    multi_select.transform_type_option(FieldType::MultiSelect, single_select.into());
    debug_assert_eq!(multi_select.options.len(), 2);
  }

  // #[test]

  #[test]
  fn multi_select_insert_multi_option_test() {
    let google = SelectOption::new("Google");
    let facebook = SelectOption::new("Facebook");
    let multi_select = MultiSelectTypeOption {
      options: vec![google.clone(), facebook.clone()],
      disable_color: false,
    };

    let option_ids = vec![google.id, facebook.id];
    let changeset = SelectOptionCellChangeset::from_insert_options(option_ids.clone());
    let select_option_ids: SelectOptionIds =
      multi_select.apply_changeset(changeset, None).unwrap().1;

    assert_eq!(&*select_option_ids, &option_ids);
  }

  #[test]
  fn multi_select_unselect_multi_option_test() {
    let google = SelectOption::new("Google");
    let facebook = SelectOption::new("Facebook");
    let multi_select = MultiSelectTypeOption {
      options: vec![google.clone(), facebook.clone()],
      disable_color: false,
    };
    let option_ids = vec![google.id, facebook.id];

    // insert
    let changeset = SelectOptionCellChangeset::from_insert_options(option_ids.clone());
    let select_option_ids = multi_select.apply_changeset(changeset, None).unwrap().1;
    assert_eq!(&*select_option_ids, &option_ids);

    // delete
    let changeset = SelectOptionCellChangeset::from_delete_options(option_ids);
    let select_option_ids = multi_select.apply_changeset(changeset, None).unwrap().1;
    assert!(select_option_ids.is_empty());
  }

  #[test]
  fn multi_select_insert_single_option_test() {
    let google = SelectOption::new("Google");
    let multi_select = MultiSelectTypeOption {
      options: vec![google.clone()],
      disable_color: false,
    };

    let changeset = SelectOptionCellChangeset::from_insert_option_id(&google.id);
    let select_option_ids = multi_select.apply_changeset(changeset, None).unwrap().1;
    assert_eq!(select_option_ids.to_string(), google.id);
  }

  #[test]
  fn multi_select_insert_non_exist_option_test() {
    let google = SelectOption::new("Google");
    let multi_select = MultiSelectTypeOption {
      options: vec![],
      disable_color: false,
    };

    let changeset = SelectOptionCellChangeset::from_insert_option_id(&google.id);
    let (_, select_option_ids) = multi_select.apply_changeset(changeset, None).unwrap();
    assert!(select_option_ids.is_empty());
  }

  #[test]
  fn multi_select_insert_invalid_option_id_test() {
    let google = SelectOption::new("Google");
    let multi_select = MultiSelectTypeOption {
      options: vec![google.clone()],
      disable_color: false,
    };

    // empty option id string
    let changeset = SelectOptionCellChangeset::from_insert_option_id("");
    let (cell_str, _) = multi_select.apply_changeset(changeset, None).unwrap();
    assert_eq!(cell_str, "");

    let changeset = SelectOptionCellChangeset::from_insert_option_id("123,456");
    let select_option_ids = multi_select.apply_changeset(changeset, None).unwrap().1;
    assert!(select_option_ids.is_empty());
  }
}
