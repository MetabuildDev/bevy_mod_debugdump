use std::{any::TypeId, path::PathBuf};

use bevy::{prelude::*, render::RenderApp, utils::HashSet};
use bevy_mod_debugdump::schedule_graph::{settings::Style, Settings};

fn main() -> Result<(), std::io::Error> {
    let docs_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs");
    let schedule_path = docs_path.join("schedule");
    let schedule_path_by_crate = schedule_path.join("by-crate");
    let render_path = docs_path.join("render");
    std::fs::create_dir_all(schedule_path.join("light"))?;
    std::fs::create_dir_all(schedule_path.join("dark"))?;
    std::fs::create_dir_all(&schedule_path_by_crate.join("light"))?;
    std::fs::create_dir_all(&schedule_path_by_crate.join("dark"))?;
    std::fs::create_dir_all(render_path.join("light"))?;
    std::fs::create_dir_all(render_path.join("dark"))?;

    let mut app = App::new();
    app.add_plugins(DefaultPlugins);

    let style_light = Style::light();
    let style_dark = Style::dark_github();

    app.world
        .resource_scope::<Schedules, _>(|world, mut schedules| {
            initialize_schedules(&mut schedules, world)?;

            let settings_light = Settings {
                style: style_light.clone(),
                ..Settings::default()
            };
            let settings_dark = Settings {
                style: style_dark.clone(),
                ..Settings::default()
            };
            for (label, schedule) in schedules.iter() {
                let dot_light = bevy_mod_debugdump::schedule_graph::schedule_graph_dot(
                    schedule,
                    &world,
                    &settings_light,
                );
                let dot_dark = bevy_mod_debugdump::schedule_graph::schedule_graph_dot(
                    schedule,
                    &world,
                    &settings_dark,
                );

                let filename = format!("schedule_{label:?}.dot");
                std::fs::write(schedule_path.join("light").join(&filename), dot_light)?;
                std::fs::write(schedule_path.join("dark").join(&filename), dot_dark)?;
            }

            // filtered main, without mass event/asset systems
            let main = schedules.get(&Main).unwrap();

            let filter = |system: &dyn System<In = (), Out = ()>| {
                let name = system.name();
                let ignore = ["asset_event_system", "update_asset_storage_system"];
                let events_update_system = name.starts_with("bevy_ecs::event::Events")
                    && name.ends_with("::update_system");
                !events_update_system && !ignore.iter().any(|remove| name.contains(remove))
            };
            let main_filtered_settings_light = Settings {
                include_system: Some(Box::new(filter)),
                style: style_light.clone(),
                ..Settings::default()
            };
            let main_filtered_settings_dark = Settings {
                include_system: Some(Box::new(filter)),
                style: style_dark.clone(),
                ..Settings::default()
            };
            let dot_light = bevy_mod_debugdump::schedule_graph::schedule_graph_dot(
                main,
                &world,
                &main_filtered_settings_light,
            );
            let dot_dark = bevy_mod_debugdump::schedule_graph::schedule_graph_dot(
                main,
                &world,
                &main_filtered_settings_dark,
            );

            let filename = format!("schedule_Main_Filtered.dot");
            std::fs::write(schedule_path.join("light").join(&filename), dot_light)?;
            std::fs::write(schedule_path.join("dark").join(&filename), dot_dark)?;

            // by crate
            let bevy_crates: HashSet<_> = main
                .graph()
                .systems()
                .filter_map(|(_, system, _)| Some(system.name().split_once("::")?.0.to_owned()))
                .collect();

            for bevy_crate in bevy_crates {
                let by_crate_settings_light = Settings {
                    style: style_light.clone(),
                    ..Default::default()
                }
                .filter_in_crate(&bevy_crate);
                let by_crate_settings_dark = Settings {
                    style: style_dark.clone(),
                    ..Default::default()
                }
                .filter_in_crate(&bevy_crate);

                let dot_light = bevy_mod_debugdump::schedule_graph::schedule_graph_dot(
                    main,
                    &world,
                    &by_crate_settings_light,
                );
                let dot_dark = bevy_mod_debugdump::schedule_graph::schedule_graph_dot(
                    main,
                    &world,
                    &by_crate_settings_dark,
                );

                let filename = format!("schedule_Main_{}.dot", bevy_crate);
                std::fs::write(
                    schedule_path_by_crate.join("light").join(&filename),
                    dot_light,
                )?;
                std::fs::write(
                    schedule_path_by_crate.join("dark").join(&filename),
                    dot_dark,
                )?;
            }

            Ok::<_, std::io::Error>(())
        })?;

    with_main_world_in_render_app(&mut app, |render_app| {
        render_app
            .world
            .resource_scope::<Schedules, _>(|world, mut schedules| {
                for (label, schedule) in schedules.iter_mut() {
                    // for access info
                    schedule.graph_mut().initialize(world);
                    // for `conflicting_systems`
                    schedule
                        .graph_mut()
                        .build_schedule(world.components())
                        .unwrap();

                    let ignore_ambiguities = &[TypeId::of::<bevy_render::texture::TextureCache>()];
                    let settings_light = Settings {
                        style: style_light.clone(),
                        ..Default::default()
                    }
                    .without_single_ambiguities_on_one_of(ignore_ambiguities);
                    let settings_dark = Settings {
                        style: style_dark.clone(),
                        ..Default::default()
                    }
                    .without_single_ambiguities_on_one_of(ignore_ambiguities);

                    let dot_light = bevy_mod_debugdump::schedule_graph::schedule_graph_dot(
                        schedule,
                        &world,
                        &settings_light,
                    );
                    let dot_dark = bevy_mod_debugdump::schedule_graph::schedule_graph_dot(
                        schedule,
                        &world,
                        &settings_dark,
                    );

                    let filename = format!("render_schedule_{label:?}.dot");
                    std::fs::write(schedule_path.join("light").join(&filename), dot_light)?;
                    std::fs::write(schedule_path.join("dark").join(&filename), dot_dark)?;
                }
                Ok::<(), std::io::Error>(())
            })
    })?;

    let settings_render_light = bevy_mod_debugdump::render_graph::Settings {
        style: bevy_mod_debugdump::render_graph::settings::Style::light(),
    };
    let settings_render_dark = bevy_mod_debugdump::render_graph::Settings {
        style: bevy_mod_debugdump::render_graph::settings::Style::dark_github(),
    };
    let dot_light = bevy_mod_debugdump::render_graph_dot(&app, &settings_render_light);
    let dot_dark = bevy_mod_debugdump::render_graph_dot(&app, &settings_render_dark);
    let filename = "render_graph.dot";
    std::fs::write(render_path.join("light").join(filename), dot_light)?;
    std::fs::write(render_path.join("dark").join(filename), dot_dark)?;

    Ok(())
}

fn initialize_schedules(
    schedules: &mut Mut<Schedules>,
    world: &mut World,
) -> Result<(), std::io::Error> {
    Ok(for (_, schedule) in schedules.iter_mut() {
        // for access info
        schedule.graph_mut().initialize(world);
        // for `conflicting_systems`
        schedule
            .graph_mut()
            .build_schedule(world.components())
            .unwrap();
    })
}

fn with_main_world_in_render_app<T>(app: &mut App, f: impl Fn(&mut App) -> T) -> T {
    // temporarily add the app world to the render world as a resource
    let inserted_world = std::mem::take(&mut app.world);
    let mut render_main_world = bevy_render::MainWorld::default();
    *render_main_world = inserted_world;

    let render_app = app.sub_app_mut(RenderApp);
    render_app.world.insert_resource(render_main_world);

    let ret = f(render_app);

    // move the app world back, as if nothing happened.
    let mut inserted_world = render_app
        .world
        .remove_resource::<bevy_render::MainWorld>()
        .unwrap();
    app.world = std::mem::take(&mut *inserted_world);

    ret
}
