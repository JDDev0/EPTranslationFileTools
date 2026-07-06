use std::collections::HashMap;
use std::env;
use std::env::ArgsOs;
use std::error::Error;
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::process::ExitCode;

fn main() -> Result<ExitCode, Box<dyn Error>> {
    let mut args = env::args_os();

    let binary_name = args.next();
    let binary_name = binary_name.as_deref().map(|str| str.display().to_string()).unwrap_or("TranslationFileLineFixer".to_string());

    let subcommand_name = args.next();
    let subcommand_name = subcommand_name.as_deref().map(|str| str.display().to_string());
    match subcommand_name.as_deref() {
        Some(subcommand @ "fix_lines") => fix_lines(&binary_name, subcommand, args),
        Some(subcommand @ "update_templates") => update_templates(&binary_name, subcommand, args),
        Some(subcommand @ "reorder_translations") => reorder_translations(&binary_name, subcommand, args),
        Some(subcommand @ "add_new_translations") => add_new_translations(&binary_name, subcommand, args),
        _ => {
            println!("\
            Usage: {binary_name} <subcommand> <args...>\n\
            Supported subcommands:\n\
            - \"fix_lines\": Adds missing whitespaces to translation files\n\
            - \"update_templates\": Combines \"block.energizedpower.\" and \"item.energizedpower.\" to \"_template.epblock.\" and \
                \"block.energizedpower.\", \"item.energizedpower.\", and \"container.energizedpower.\" to \"_template.epcontainer.\"\n\
            - \"reorder_translations\": Reorders translations from translation type based order to functionality based order \
                (e.g. item name, tooltips, advancement, and book page are directly below the item translation)\n\
            - \"add_new_translations\": Adds new translations from a reference file");

            Ok(ExitCode::FAILURE)
        }
    }
}

fn fix_lines(binary_name: &str, subcommand: &str, args: ArgsOs) -> Result<ExitCode, Box<dyn Error>> {
    let args = args.collect::<Vec<OsString>>();
    let args = args.as_array();

    let Some([reference, input, output]) = args else {
        println!("Usage: {binary_name} {subcommand} <reference file> <input file> <output file>");

        return Ok(ExitCode::FAILURE);
    };

    let reference = File::open(reference)?;
    let input = File::open(input)?;
    let mut output = File::create(output)?;

    let reference = BufReader::new(reference).lines();
    let mut input = BufReader::new(input).lines();

    for line in reference {
        let line = line?;

        if line.trim().is_empty() {
            writeln!(output)?;
        }else if let Some(input) = input.next() {
            let input = input?;
            writeln!(output, "{input}")?;
        }
    }

    println!("Translation file was fixed!");

    Ok(ExitCode::SUCCESS)
}

fn update_templates(binary_name: &str, subcommand: &str, args: ArgsOs) -> Result<ExitCode, Box<dyn Error>> {
    let args = args.collect::<Vec<OsString>>();
    let args = args.as_array();

    let Some([input, output]) = args else {
        println!("Usage: {binary_name} {subcommand} <input file> <output file>");

        return Ok(ExitCode::FAILURE);
    };

    let input = File::open(input)?;
    let mut output = File::create(output)?;

    let mut line_buffer = Vec::new();
    let mut has_item = false;
    let mut has_block = false;
    let mut has_container = false;

    fn write_and_clear_combined_translations_line_buffer(
        output: &mut File, line_buffer: &mut Vec<String>,
        has_item: &mut bool, has_block: &mut bool, has_container: &mut bool,
    ) -> Result<(), Box<dyn Error>> {
        if *has_item && *has_block {
            let line_with_block_key = &**line_buffer.iter().find(|line| line.contains("\"block.energizedpower.")).unwrap();
            let line = if *has_container {
                line_with_block_key.replace("\"block.energizedpower.", "\"_template.epcontainer.")
            }else {
                line_with_block_key.replace("\"block.energizedpower.", "\"_template.epblock.")
            };

            line_buffer.clear();
            writeln!(output, "{line}")?;
        }else {
            for line in line_buffer.drain(..) {
                writeln!(output, "{line}")?;
            }
        }

        *has_item = false;
        *has_block = false;
        *has_container = false;

        Ok(())
    }

    let input = BufReader::new(input).lines();
    for line in input {
        let line = line?;

        if line.contains("\"item.energizedpower.") {
            if has_item {
                write_and_clear_combined_translations_line_buffer(
                    &mut output, &mut line_buffer,
                    &mut has_item, &mut has_block, &mut has_container,
                )?;
            }

            has_item = true;
            line_buffer.push(line);
        }else if line.contains("\"block.energizedpower.") {
            if has_block {
                write_and_clear_combined_translations_line_buffer(
                    &mut output, &mut line_buffer,
                    &mut has_item, &mut has_block, &mut has_container,
                )?;
            }

            has_block = true;
            line_buffer.push(line);
        }else if line.contains("\"container.energizedpower.") &&
                //Ignore special case: Separate translations for Item Conveyor Belt machine containers
                !line.contains("item_conveyor_belt") {
            if has_container {
                write_and_clear_combined_translations_line_buffer(
                    &mut output, &mut line_buffer,
                    &mut has_item, &mut has_block, &mut has_container,
                )?;
            }

            has_container = true;
            line_buffer.push(line);
        }else {
            if has_item || has_block || has_container {
                write_and_clear_combined_translations_line_buffer(
                    &mut output, &mut line_buffer,
                    &mut has_item, &mut has_block, &mut has_container,
                )?;
            }

            writeln!(output, "{line}")?;
        }
    }

    write_and_clear_combined_translations_line_buffer(
        &mut output, &mut line_buffer,
        &mut has_item, &mut has_block, &mut has_container,
    )?;

    println!("Translation file was updated!");

    Ok(ExitCode::SUCCESS)
}

fn reorder_translations(binary_name: &str, subcommand: &str, args: ArgsOs) -> Result<ExitCode, Box<dyn Error>> {
    let args = args.collect::<Vec<OsString>>();
    if args.len() != 1 && args.len() != 2 {
        println!("Usage: {binary_name} {subcommand} <input file> <output file>");
        println!("Usage (inplace update): {binary_name} {subcommand} <file>");

        return Ok(ExitCode::FAILURE);
    }

    let input = &args[0];
    let output = args.get(1).unwrap_or(input);

    let input = File::open(input)?;

    let input = BufReader::new(input).lines();
    let input = input.collect::<Result<Vec<_>,_>>()?;

    let mut output = File::create(output)?;

    const TRANSLATION_TYPE_PREFIXES: [&str; 27] = [
        "item.energizedpower.",
        "block.energizedpower.",
        "block_state.energizedpower.",
        "fluid_type.energizedpower.",
        "container.energizedpower.",
        "_template.item.energizedpower.",
        "_template.block.energizedpower.",
        "_template.epblock.",
        "_template.epcontainer.",
        "tooltip.energizedpower.machine_configuration.",
        "tooltip.energizedpower.",
        "txt.energizedpower.",
        "entity.energizedpower.",
        "soil_type.energizedpower.",
        "recipes.energizedpower.",
        "emi.category.energizedpower.",
        "painting.energizedpower.",
        "advancements.energizedpower.",
        "_template.advancements.energizedpower.",
        "key.energizedpower.",
        "book.energizedpower.page.chapter.",
        "book.energizedpower.page.",
        "book.energizedpower.",
        "itemGroup.energizedpower.",

        "entity.minecraft.villager.",

        "_template.community_translations.info.",
        "_template.cti.",
    ];

    let mut dummy_functionality = 0;
    let mut functionality_order = Vec::new();
    let mut functionality_translation_map = HashMap::new();

    fn unify_and_separate_functionalities(line: &str, functionality: String) -> String {
        //Separate paintings
        if line.contains("\"painting") {
            return "painting".to_string() + &functionality;
        }

        //Unify soil types
        if line.contains("\"soil_type") {
            return "UNIFIED_soil_type".to_string();
        }

        //Handle community translations info
        if line.contains("\"_template.community_translations.info.") || line.contains("\"_template.cti.") {
            if line.contains("\"_template.community_translations.info.") {
                //Unify info.1 and info.2 with "contribution"
                if line.contains("info.1\"") || line.contains("info.2\"") {
                    return "CTI_contribution".to_string();
                }

                //Unify info.3 with "datagen"
                if line.contains("info.3\"") {
                    return "CTI_datagen".to_string();
                }
            }

            //Force separate from other translations
            return "CTI_".to_string() + &functionality;
        }

        const UNIFIED_FUNCTIONALITIES: [&str; 27] = [
            "fertilizer",

            "press_mold",

            "dust",
            "nugget",
            "gear",
            "rod",

            "dirty_water",
            "liquid_xp",

            "speed_upgrade",
            "energy_efficiency_upgrade",
            "energy_production_upgrade",
            "energy_capacity_upgrade",
            "duration_upgrade",
            "extraction_range_upgrade",
            "range_upgrade",
            "extraction_depth_upgrade",
            "moon_light_upgrade",
            "item_ejector_upgrade",
            "item_pulling_upgrade",

            "hammer",
            "cutter",

            "ehv_transformer",
            "hv_transformer",
            "lv_transformer",
            "mv_transformer", "transformer_",

            "solar_panel",
        ];

        for unified_functionality in UNIFIED_FUNCTIONALITIES {
            if functionality.contains(unified_functionality) {
                //Do not unify fertilizers and plant growth chamber fertilizer recipe
                if unified_functionality == "fertilizer" && functionality.contains("plant_growth_chamber") {
                    continue;
                }

                //Do not unify press molds and press mold maker
                if unified_functionality == "press_mold" && functionality.contains("maker") {
                    continue;
                }

                //Do unify raw press molds and cooked press molds separately
                if unified_functionality == "press_mold" && functionality.contains("raw") {
                    return "UNIFIED_raw_press_mold".to_string();
                }

                //Do not unify metal dusts and sawdust or charcoal dust
                if unified_functionality == "dust" && (functionality.contains("sawdust") || functionality.contains("charcoal")) {
                    continue;
                }

                //Do not unify metal rods and products or production (contains "rod")
                if unified_functionality == "rod" && functionality.contains("prod") {
                    continue;
                }

                //Do not unify cutter and auto stonecutter
                if unified_functionality == "cutter" && functionality.contains("stonecutter") {
                    continue;
                }

                //Do not unify mv transformers with transformer connection block state
                if unified_functionality == "transformer_" {
                    if functionality == "transformer_connection" {
                        continue;
                    }

                    return "UNIFIED_mv_transformer".to_string();
                }

                return "UNIFIED_".to_string() + unified_functionality;
            }
        }

        //Unify battery items
        if functionality == "battery" || functionality.strip_prefix("battery_").is_some_and(|suffix| suffix.parse::<u8>().is_ok()) ||
                functionality == "battery_8_fully_charged" || functionality == "batteries" {
            return "UNIFIED_battery".to_string();
        }

        //Unify plant growth chamber recipes with plant growth chamber
        if functionality.contains("plant_growth_chamber_") {
            return "plant_growth_chamber".to_string();
        }

        //Unify some general tooltips
        if matches!(&*functionality,
            "energy_meter" | "fluid_meter" |
            "energy_consumption_per_tick" | "energy_production_per_tick" |
            "recipe" |
            "shift_details" |
            "capacity" |
            "transfer_rate" | "tank_capacity" |
            "not_enough_energy" |
            "ready" |
            "infinite"
        ) {
            return "UNIFIED_general_tooltips".to_string();
        }

        //Unify "iron_fluid_pipe" with "fluid_pipe"
        if functionality == "iron_fluid_pipe" {
            return "fluid_pipe".to_string();
        }

        //Separate iron fluid pipe and fluid pipe tooltips
        if functionality == "fluid_pipe" && line.contains("tooltip") {
            return "fluid_pipe_tooltip".to_string();
        }

        //Separate auto crafters and auto crafter tooltips
        if functionality == "auto_crafter" && line.contains("tooltip") {
            return "auto_crafter_tooltip".to_string();
        }

        //Unify charger emi category and book page
        if functionality == "charger" && line.contains("emi.category") {
            return "chargers".to_string();
        }

        //Unify crusher emi category and book page
        if functionality == "crusher" && line.contains("emi.category") {
            return "crushers".to_string();
        }

        functionality
    }

    //Group by functionality while preserving relative order
    for line in input {
        if line.trim().is_empty() {
            continue;
        }

        let translation_type_prefix = TRANSLATION_TYPE_PREFIXES.into_iter().find(|prefix| line.contains(&("\"".to_string() + prefix)));
        if let Some(translation_type_prefix) = translation_type_prefix {
            let translation_line_without_prefix = &line[line.find(translation_type_prefix).unwrap() + translation_type_prefix.len()..];

            let functionality_index_dot = translation_line_without_prefix.find(".");
            let functionality_index_double_quote = translation_line_without_prefix.find("\"");
            let functionality_end_index = match (functionality_index_dot, functionality_index_double_quote) {
                (Some(index_dot), Some(index_double_quote)) => index_dot.min(index_double_quote),
                (Some(index_dot), None) => index_dot,
                (None, Some(index_double_quote)) => index_double_quote,
                (None, None) => panic!("Invalid JSON syntax in translation file for line:\n{line}"),
            };

            let functionality = unify_and_separate_functionalities(&line, translation_line_without_prefix[..functionality_end_index].to_string());

            let functionality_translations_entry = functionality_translation_map.entry(functionality.clone());
            let functionality_translations = functionality_translations_entry.or_insert_with(|| {
                functionality_order.push(functionality);
                 Vec::new()
            });

            functionality_translations.push(line);
        }else {
            let functionality_key = format!("DUMMY_{dummy_functionality}");
            functionality_order.push(functionality_key.clone());
            functionality_translation_map.insert(functionality_key, vec![line]);

            dummy_functionality += 1;
        }
    }

    //Improve functionality order
    {
        const MOVE_AFTER: [(&str, &str); 22] = [
            //Cable tooltip and book page after highest tier cable
            ("cable", "energized_crystal_matrix_cable"),
            ("cables", "cable"),

            //Transformer tooltips and book page after highest tier transformer
            ("transformer", "UNIFIED_ehv_transformer"),
            ("configurable_transformer", "transformer"),
            ("transformers", "configurable_transformer"),

            //Special upgrade module book page after last upgrade module of the special upgrade module type
            ("furnace_mode_upgrades", "smoker_upgrade_module"),

            //Book page after highest tier item
            ("circuits", "quantum_processing_unit"),
            ("solar_cells", "elite_solar_cell"),

            //Book page after highest tier machine
            ("machine_frames", "reinforced_advanced_machine_frame"),
            ("powered_furnaces", "advanced_powered_furnace"),
            ("auto_crafters", "advanced_auto_crafter"),
            ("crushers", "advanced_crusher"),
            ("pulverizers", "advanced_pulverizer"),
            ("fluid_pumps", "advanced_fluid_pump"),
            ("fluid_pipes", "fluid_pipe_tooltip"),
            ("item_silos", "item_silo_capacity"),

            //Combined book page after highest tier machine
            ("charger_uncharger", "advanced_uncharger"),
            ("minecart_charger_uncharger", "advanced_minecart_uncharger"),
            ("fluid_filler_fluid_drainer", "fluid_drainer"),

            //Book page after highest tier entity
            ("battery_box_minecarts", "advanced_battery_box_minecart"),

            //Tooltips after highest tier machine
            ("auto_crafter_tooltip", "advanced_auto_crafter"),

            //Tooltips and recipe category after highest tier machine
            ("chargers", "advanced_charger"),
        ];

        for (from, to) in MOVE_AFTER {
            let index_from = functionality_order.iter().enumerate().
                    find(|(_, functionality)| *functionality == from).
                    map(|(index, _)| index);

            let index_to = functionality_order.iter().enumerate().
                    find(|(_, functionality)| *functionality == to).
                    map(|(index, _)| index);

            if let Some(index_from) = index_from && let Some(mut index_to) = index_to {
                if index_from > index_to {
                    index_to += 1;
                }

                let functionality = functionality_order.remove(index_from);
                functionality_order.insert(index_to, functionality);
            }
        }
    }

    //Write all reordered translations to output
    let functionality_count = functionality_order.len();
    for (index, functionality) in functionality_order.into_iter().enumerate() {
        let mut should_append_whitespace = true;
        let functionality_translations = &*functionality_translation_map[&functionality];
        for line in functionality_translations {
            writeln!(output, "{line}")?;

            if line.trim() == "{" || index == functionality_count - 2 || line.trim() == "}" {
                should_append_whitespace = false;
            }
        }

        if should_append_whitespace {
            writeln!(output)?;
        }

        //Add 2 additional empty lines after community translations info
        if functionality == "CTI_datagen" {
            writeln!(output)?;
            writeln!(output)?;
        }
    }

    println!("Translation file was reordered!");

    Ok(ExitCode::SUCCESS)
}

fn add_new_translations(binary_name: &str, subcommand: &str, args: ArgsOs) -> Result<ExitCode, Box<dyn Error>> {
    let args = args.collect::<Vec<OsString>>();
    if args.len() != 2 && args.len() != 3 {
        println!("Usage: {binary_name} {subcommand} <reference file> <input file> <output file>");
        println!("Usage (inplace update): {binary_name} {subcommand} <reference file> <file>");

        return Ok(ExitCode::FAILURE);
    }

    let reference = &args[0];
    let input = &args[1];
    let output = args.get(2).unwrap_or(input);

    let reference = File::open(reference)?;
    let mut reference = BufReader::new(reference).lines();

    let input = File::open(input)?;
    let input = BufReader::new(input).lines();
    let input = input.collect::<Result<Vec<_>,_>>()?;

    let mut output = File::create(output)?;

    fn translation_key_equals(line_a: &str, line_b: &str) -> bool {
        if line_a == line_b {
            return true;
        }

        if let Some(index_a) = line_a.find("\":") && let Some(index_b) = line_b.find("\":") {
            let key_a = &line_a[..index_a];
            let key_b = &line_b[..index_b];

            return key_a == key_b;
        }

        false
    }

    for line in input {
        while let Some(reference_line) = reference.next() &&
                let reference_line = reference_line? &&
                !translation_key_equals(&line, &reference_line) {
            writeln!(output, "{reference_line}")?;
        }

        writeln!(output, "{line}")?;
    }

    println!("Translation file was extended with new translations!");

    Ok(ExitCode::SUCCESS)
}
