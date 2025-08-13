use locks::RwLock;
use skyline::hooks::InlineCtx;
use smash::app::BattleObject;
use smashline::{BattleObjectCategory, Costume, Hash40, L2CFighterBase, ObjectEvent};

pub type StateCallbackFunction = unsafe extern "C" fn(&mut L2CFighterBase);

pub struct StateCallback {
    pub agent: Option<Hash40>,
    pub event: ObjectEvent,
    pub function: StateCallbackFunction,
    pub costume: Costume,
}

pub static STATE_CALLBACKS: RwLock<Vec<StateCallback>> = RwLock::new(Vec::new());

fn call_state_callback(agent: &mut L2CFighterBase, event: ObjectEvent) {
    let hash = crate::create_agent::agent_hash(agent);
    let callbacks = STATE_CALLBACKS.read();

    let object: &mut BattleObject = unsafe {std::mem::transmute(agent.battle_object)};
    let category = BattleObjectCategory::from_battle_object_id(object.battle_object_id);

    let costume = crate::utils::get_agent_costume(agent.battle_object as *const BattleObject).unwrap_or(0);
    let has_costume = crate::utils::has_costume(hash, costume);

    for callback in callbacks.iter().filter(|cb| cb.event == event) {
        if let Some(required) = callback.agent {
            if hash != required {
                match category {
                    Some(BattleObjectCategory::Fighter) => if required != Hash40::new("fighter") { continue; },
                    Some(BattleObjectCategory::Weapon) => if required != Hash40::new("weapon") { continue; },
                    _ => { continue; },
                }
            } else {
                let c = callback.costume.as_slice();

                if has_costume && !c.contains(&(costume as usize)) {
                    continue;
                }

                if !has_costume && !c.is_empty() {
                    continue;
                }
            }
        }

        unsafe {
            (callback.function)(agent);
        }
    }
}

pub static mut CAN_RUN_ON_START : bool = false;

#[skyline::hook(offset = 0x48ad04, inline)]
unsafe fn lua_module_start_lua2cpp(_ctx: &InlineCtx) {
    CAN_RUN_ON_START = true;
}

#[skyline::hook(offset = 0x48ada0)]
unsafe fn lua_module_end(lua_module: *const u64) {
    let agent = std::mem::transmute(*lua_module.add(0x1d8 / 8));
    call_state_callback(agent, ObjectEvent::End);
}

#[skyline::hook(offset = 0x48abac, inline)]
unsafe fn lua_module_initialize_lua2cpp(ctx: &InlineCtx) {
    let agent = std::mem::transmute(ctx.registers[0].x());
    call_state_callback(agent, ObjectEvent::Initialize);
}

#[skyline::hook(offset = 0x48ac44, inline)]
unsafe fn lua_module_finalize_lua2cpp(ctx: &InlineCtx) {
    let agent = std::mem::transmute(ctx.registers[0].x());
    call_state_callback(agent, ObjectEvent::Finalize);
}

#[skyline::hook(offset = 0x3afde0, inline)]
unsafe fn start_module_accessor_end(ctx: &mut InlineCtx) {
    if CAN_RUN_ON_START {
        let boma = ctx.registers[19].x();
        let lua_module = *(boma as *mut u64).add(0x190 / 8);
        let agent = std::mem::transmute(*((lua_module + 0x1D8) as *mut *mut L2CFighterBase));
        call_state_callback(agent, ObjectEvent::Start);
    }
    CAN_RUN_ON_START = false;
}

pub fn install_state_callback_hooks() {
    skyline::install_hooks!(
        lua_module_start_lua2cpp,
        lua_module_end,
        lua_module_initialize_lua2cpp,
        lua_module_finalize_lua2cpp,

        start_module_accessor_end,
    );
}
