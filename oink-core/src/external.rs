//! External functions bound into every story.

use crate::engine::{CheckRecord, EngineError, Shared};
use bladeink::story::external_functions::{ExternalFunctionError, ExternalFunctionResult};
use bladeink::story::Story;
use bladeink::value_type::ValueType;
use oink_rulebook::{CheckRequest, Checks};
use std::rc::Rc;

pub(crate) fn bind_external_functions(
    story: &mut Story,
    state: &Shared,
) -> Result<(), EngineError> {
    {
        let state = Rc::clone(state);
        bind(story, "roll_check", false, move |_name, args| {
            let ability = arg_string(args, 0);
            let tags = parse_tags(&arg_string(args, 2));
            let tag_refs: Vec<&str> = tags.iter().map(String::as_str).collect();
            let request = CheckRequest {
                ability: ability.as_str(),
                difficulty: arg_int(args, 1),
                tags: &tag_refs,
                modifier: arg_int(args, 3),
            };
            let mut guard = state.borrow_mut();
            let state = &mut *guard;
            let checks = Checks::new(&state.rulebook, &state.character);
            let result = checks
                .active(state.dice.as_mut(), &request)
                .map_err(external_error)?;
            state.checks.push(CheckRecord {
                ability: request.ability.to_string(),
                difficulty: request.difficulty,
                outcome: result.outcome.as_str().to_string(),
                total: result.total,
                dice: Some(result.dice),
                breakdown: result.breakdown,
            });
            string_result(result.outcome.as_str())
        })?;
    }

    {
        let state = Rc::clone(state);
        bind(story, "passive_check", false, move |_name, args| {
            let owned = request_from_args(args, true);
            let tag_refs: Vec<&str> = owned.tags.iter().map(String::as_str).collect();
            let request = CheckRequest {
                ability: &owned.ability,
                difficulty: owned.difficulty,
                tags: &tag_refs,
                modifier: owned.modifier,
            };
            let mut guard = state.borrow_mut();
            let state = &mut *guard;
            let checks = Checks::new(&state.rulebook, &state.character);
            let result = checks.passive(&request).map_err(external_error)?;
            state.checks.push(CheckRecord {
                ability: request.ability.to_string(),
                difficulty: request.difficulty,
                outcome: if result.passed {
                    "pass".to_string()
                } else {
                    "fail".to_string()
                },
                total: result.value,
                dice: None,
                breakdown: result.breakdown,
            });
            int_result(i32::from(result.passed))
        })?;
    }

    {
        let state = Rc::clone(state);
        bind(story, "passive_value", true, move |_name, args| {
            let owned = request_from_args(args, false);
            let tag_refs: Vec<&str> = owned.tags.iter().map(String::as_str).collect();
            let request = CheckRequest {
                ability: &owned.ability,
                difficulty: owned.difficulty,
                tags: &tag_refs,
                modifier: owned.modifier,
            };
            let guard = state.borrow();
            let checks = Checks::new(&guard.rulebook, &guard.character);
            let result = checks.passive(&request).map_err(external_error)?;
            int_result(result.value)
        })?;
    }

    {
        let state = Rc::clone(state);
        bind(story, "check_breakdown", true, move |_name, args| {
            let owned = request_from_args(args, false);
            let tag_refs: Vec<&str> = owned.tags.iter().map(String::as_str).collect();
            let request = CheckRequest {
                ability: &owned.ability,
                difficulty: owned.difficulty,
                tags: &tag_refs,
                modifier: owned.modifier,
            };
            let guard = state.borrow();
            let checks = Checks::new(&guard.rulebook, &guard.character);
            let breakdown = checks
                .breakdown(request.ability, request.tags, request.modifier)
                .map_err(external_error)?;
            string_result(&breakdown.to_string())
        })?;
    }

    {
        let state = Rc::clone(state);
        bind(story, "end_scene", false, move |_name, _args| {
            let mut guard = state.borrow_mut();
            let changes = guard.character.on_scene_end();
            guard.changes.extend(changes);
            void_result()
        })?;
    }

    {
        let state = Rc::clone(state);
        bind(story, "difficulty", true, move |_name, args| {
            let name = arg_string(args, 0);
            let guard = state.borrow();
            let value = guard
                .rulebook
                .difficulties
                .get(&name)
                .copied()
                .ok_or_else(|| external_error(format!("unknown difficulty `{name}`")))?;
            int_result(value)
        })?;
    }

    {
        let state = Rc::clone(state);
        bind(story, "enter_environment", false, move |_name, args| {
            let id = arg_string(args, 0);
            let mut guard = state.borrow_mut();
            let state = &mut *guard;
            ensure(
                state.rulebook.environments.contains_key(&id),
                format!("unknown environment `{id}`"),
            )?;
            let changes = state.character.enter_environment(&state.rulebook, &id);
            state.changes.extend(changes);
            void_result()
        })?;
    }

    {
        let state = Rc::clone(state);
        bind(story, "clear_environment", false, move |_name, args| {
            let id = arg_string(args, 0);
            let mut guard = state.borrow_mut();
            let state = &mut *guard;
            let changes = state.character.clear_environment(&id);
            state.changes.extend(changes);
            void_result()
        })?;
    }

    {
        let state = Rc::clone(state);
        bind(story, "has_item", true, move |_name, args| {
            let guard = state.borrow();
            bool_result(guard.character.has_item(&arg_string(args, 0)))
        })?;
    }

    {
        let state = Rc::clone(state);
        bind(story, "add_item", false, move |_name, args| {
            let id = arg_string(args, 0);
            let mut guard = state.borrow_mut();
            let state = &mut *guard;
            ensure(
                state.rulebook.items.contains_key(&id),
                format!("unknown item `{id}`"),
            )?;
            let changes = state.character.add_item(&state.rulebook, &id);
            state.changes.extend(changes);
            void_result()
        })?;
    }

    {
        let state = Rc::clone(state);
        bind(story, "remove_item", false, move |_name, args| {
            let id = arg_string(args, 0);
            let mut guard = state.borrow_mut();
            let state = &mut *guard;
            let changes = state.character.remove_item(&id);
            state.changes.extend(changes);
            void_result()
        })?;
    }

    {
        let state = Rc::clone(state);
        bind(story, "use_item", false, move |_name, args| {
            let id = arg_string(args, 0);
            let mut guard = state.borrow_mut();
            let state = &mut *guard;
            let item = state
                .rulebook
                .items
                .get(&id)
                .ok_or_else(|| external_error(format!("unknown item `{id}`")))?;
            let consumable = item.consumable;
            let conditions = item.applies_conditions.clone();
            if consumable && state.character.has_item(&id) {
                for condition in conditions {
                    let changes = state.character.add_condition(&state.rulebook, &condition);
                    state.changes.extend(changes);
                }
                let changes = state.character.remove_item(&id);
                state.changes.extend(changes);
            }
            void_result()
        })?;
    }

    {
        let state = Rc::clone(state);
        bind(story, "has_perk", true, move |_name, args| {
            let guard = state.borrow();
            bool_result(guard.character.has_perk(&arg_string(args, 0)))
        })?;
    }

    {
        let state = Rc::clone(state);
        bind(story, "add_perk", false, move |_name, args| {
            let id = arg_string(args, 0);
            let mut guard = state.borrow_mut();
            let state = &mut *guard;
            ensure(
                state.rulebook.perks.contains_key(&id),
                format!("unknown perk `{id}`"),
            )?;
            let changes = state.character.add_perk(&state.rulebook, &id);
            state.changes.extend(changes);
            void_result()
        })?;
    }

    {
        let state = Rc::clone(state);
        bind(story, "remove_perk", false, move |_name, args| {
            let id = arg_string(args, 0);
            let mut guard = state.borrow_mut();
            let state = &mut *guard;
            let changes = state.character.remove_perk(&id);
            state.changes.extend(changes);
            void_result()
        })?;
    }

    {
        let state = Rc::clone(state);
        bind(story, "has_condition", true, move |_name, args| {
            let guard = state.borrow();
            bool_result(guard.character.has_condition(&arg_string(args, 0)))
        })?;
    }

    {
        let state = Rc::clone(state);
        bind(story, "add_condition", false, move |_name, args| {
            let id = arg_string(args, 0);
            let mut guard = state.borrow_mut();
            let state = &mut *guard;
            ensure(
                state.rulebook.conditions.contains_key(&id),
                format!("unknown condition `{id}`"),
            )?;
            let changes = state.character.add_condition(&state.rulebook, &id);
            state.changes.extend(changes);
            void_result()
        })?;
    }

    {
        let state = Rc::clone(state);
        bind(story, "remove_condition", false, move |_name, args| {
            let id = arg_string(args, 0);
            let mut guard = state.borrow_mut();
            let state = &mut *guard;
            let changes = state.character.remove_condition(&id);
            state.changes.extend(changes);
            void_result()
        })?;
    }

    {
        let state = Rc::clone(state);
        bind(story, "has_tag", true, move |_name, args| {
            let tag = arg_string(args, 0);
            let guard = state.borrow();
            bool_result(guard.character.has_tag(&guard.rulebook, &tag))
        })?;
    }

    {
        let state = Rc::clone(state);
        bind(story, "ability_level", true, move |_name, args| {
            let guard = state.borrow();
            int_result(guard.character.ability_level(&arg_string(args, 0)))
        })?;
    }

    {
        let state = Rc::clone(state);
        bind(story, "characteristic", true, move |_name, args| {
            let id = arg_string(args, 0);
            let guard = state.borrow();
            let value = guard
                .character
                .characteristic(&id)
                .ok_or_else(|| external_error(format!("unknown characteristic `{id}`")))?;
            int_result(value)
        })?;
    }

    {
        let state = Rc::clone(state);
        bind(story, "characteristic_bonus", true, move |_name, args| {
            let id = arg_string(args, 0);
            let guard = state.borrow();
            let value = guard.character.characteristic(&id).unwrap_or(0);
            let bonus = guard
                .rulebook
                .characteristics
                .get(&id)
                .map(|definition| definition.bonus_for(value))
                .ok_or_else(|| external_error(format!("unknown characteristic `{id}`")))?;
            int_result(bonus)
        })?;
    }

    {
        let state = Rc::clone(state);
        bind(story, "resource", true, move |_name, args| {
            let id = arg_string(args, 0);
            let guard = state.borrow();
            let value = guard
                .character
                .resource(&id)
                .ok_or_else(|| external_error(format!("unknown resource `{id}`")))?;
            int_result(value)
        })?;
    }

    {
        let state = Rc::clone(state);
        bind(story, "spend_resource", false, move |_name, args| {
            let id = arg_string(args, 0);
            let amount = arg_int(args, 1);
            let mut guard = state.borrow_mut();
            let state = &mut *guard;
            ensure(
                state.rulebook.resources.contains_key(&id),
                format!("unknown resource `{id}`"),
            )?;
            ensure(
                amount >= 0,
                "resource amount must be nonnegative".to_string(),
            )?;
            let paid = state
                .character
                .can_spend_resource(&state.rulebook, &id, amount);
            let changes = state.character.spend_resource(&state.rulebook, &id, amount);
            state.changes.extend(changes);
            bool_result(paid)
        })?;
    }

    {
        let state = Rc::clone(state);
        bind(story, "restore_resource", false, move |_name, args| {
            let id = arg_string(args, 0);
            let amount = arg_int(args, 1);
            let mut guard = state.borrow_mut();
            let state = &mut *guard;
            ensure(
                state.rulebook.resources.contains_key(&id),
                format!("unknown resource `{id}`"),
            )?;
            ensure(
                amount >= 0,
                "resource amount must be nonnegative".to_string(),
            )?;
            let changes = state
                .character
                .restore_resource(&state.rulebook, &id, amount);
            let changed = !changes.is_empty();
            state.changes.extend(changes);
            bool_result(changed)
        })?;
    }

    #[cfg(feature = "spells")]
    {
        let state = Rc::clone(state);
        bind(story, "cast_spell", false, move |_name, args| {
            let id = arg_string(args, 0);
            let mut guard = state.borrow_mut();
            let state = &mut *guard;
            let result = oink_rulebook::spell::cast(
                &state.rulebook,
                &mut state.character,
                &id,
                state.dice.as_mut(),
            )
            .map_err(external_error)?;
            let spell = &state.rulebook.spells[&id];
            let difficulty = spell
                .check
                .difficulty
                .resolve(&state.rulebook)
                .expect("cast validated difficulty");
            state.checks.push(CheckRecord {
                ability: spell.ability.clone(),
                difficulty,
                outcome: result.outcome.as_str().to_string(),
                total: result.check.total,
                dice: Some(result.check.dice),
                breakdown: result.check.breakdown,
            });
            state.changes.extend(result.changes);
            string_result(result.outcome.as_str())
        })?;
    }

    Ok(())
}

fn bind<F>(
    story: &mut Story,
    name: &str,
    lookahead_safe: bool,
    mut function: F,
) -> Result<(), EngineError>
where
    F: FnMut(&str, &[ValueType]) -> ExternalFunctionResult + 'static,
{
    story
        .bind_external_function(
            name,
            move |name, args| {
                let signature = match name {
                    "roll_check" | "passive_check" => "sisi",
                    "passive_value" | "check_breakdown" => "ssi",
                    "spend_resource" | "restore_resource" => "si",
                    "end_scene" => "",
                    _ => "s",
                };
                if args.len() != signature.len() {
                    return Err(external_error(format!(
                        "{name} expects {} arguments",
                        signature.len()
                    )));
                }
                for (index, (argument, expected)) in args.iter().zip(signature.chars()).enumerate()
                {
                    let valid = matches!(
                        (expected, argument),
                        ('s', ValueType::String(_)) | ('i', ValueType::Int(_))
                    );
                    if !valid {
                        let kind = if expected == 's' { "string" } else { "integer" };
                        return Err(external_error(format!(
                            "{name} argument {} must be {kind}",
                            index + 1
                        )));
                    }
                }
                function(name, args)
            },
            lookahead_safe,
        )
        .map_err(EngineError::from)
}

fn request_from_args(args: &[ValueType], has_difficulty: bool) -> OwnedRequest {
    let ability = arg_string(args, 0);
    let difficulty = if has_difficulty { arg_int(args, 1) } else { 0 };
    let tags_index = if has_difficulty { 2 } else { 1 };
    let tags = parse_tags(&arg_string(args, tags_index));
    let modifier = arg_int(args, tags_index + 1);
    OwnedRequest {
        ability,
        difficulty,
        tags,
        modifier,
    }
}

/// Owns its strings so a `CheckRequest` can borrow from it.
struct OwnedRequest {
    ability: String,
    difficulty: i32,
    tags: Vec<String>,
    modifier: i32,
}

fn external_error(error: impl std::fmt::Display) -> ExternalFunctionError {
    ExternalFunctionError::new(error.to_string())
}

fn ensure(condition: bool, message: String) -> ExternalFunctionResult {
    if condition {
        void_result()
    } else {
        Err(external_error(message))
    }
}

fn arg_string(args: &[ValueType], index: usize) -> String {
    match args.get(index) {
        Some(ValueType::String(value)) => value.string.clone(),
        Some(ValueType::Int(value)) => value.to_string(),
        _ => String::new(),
    }
}

fn arg_int(args: &[ValueType], index: usize) -> i32 {
    match args.get(index) {
        Some(ValueType::Int(value)) => *value,
        _ => 0,
    }
}

fn parse_tags(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_string)
        .collect()
}

fn int_result(value: i32) -> ExternalFunctionResult {
    Ok(Some(ValueType::new(value)))
}

fn bool_result(value: bool) -> ExternalFunctionResult {
    Ok(Some(ValueType::new(value)))
}

fn string_result(value: &str) -> ExternalFunctionResult {
    Ok(Some(ValueType::from(value)))
}

fn void_result() -> ExternalFunctionResult {
    Ok(None)
}
