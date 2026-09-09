//! `apply`: validate, then mutate. Each command handler checks every precondition before it
//! touches the state, so a refused command leaves the state exactly as it was.

use cl_model::{Board, Cover, TileId, UnitKind};

use crate::command::{Command, Event, RuleError};
use crate::constants::{
    CAPITAL_GOLD, CAPITAL_WHEAT, EMPTY_WHEAT, FIELD_BASE_COST, FIELD_WHEAT,
    KNIGHT_UPGRADE_BASE_COST, KNIGHT_UPGRADE_STEP, PAWN_BASE_COST, TOWN_BASE_COST,
    WARRIOR_UPGRADE_BASE_COST,
};
use crate::state::{GameState, Territory, Treasury, Unit, territories, territory_of};

/// Applies `cmd` for the current faction. On `Err` the state is unchanged.
pub fn apply(
    state: &mut GameState,
    board: &dyn Board,
    cmd: Command,
) -> Result<Vec<Event>, RuleError> {
    if state.winner.is_some() {
        return Err(RuleError::GameOver);
    }
    match cmd {
        Command::EndTurn => Ok(end_turn(state, board)),
        Command::BuildField { at } => build(state, board, at, Cover::Field),
        Command::BuildTown { at } => build(state, board, at, Cover::Town),
        Command::RecruitPawn { at } => recruit(state, board, at),
        Command::Upgrade { at } => upgrade(state, board, at),
        Command::MoveUnit { .. } => Err(RuleError::NotYetImplemented("R-UNIT-04")),
    }
}

fn check_tile(state: &GameState, id: TileId) -> Result<(), RuleError> {
    if id.index() >= state.tiles.len() {
        return Err(RuleError::NoSuchTile(id));
    }
    if !state.tile(id).terrain.is_land() {
        return Err(RuleError::NotLand(id));
    }
    Ok(())
}

/// The current faction's territory around `id`, with a capital to pay from.
fn own_territory(
    state: &GameState,
    board: &dyn Board,
    id: TileId,
) -> Result<(Territory, TileId), RuleError> {
    check_tile(state, id)?;
    if state.tile(id).owner != Some(state.current) {
        return Err(RuleError::NotOwned(id));
    }
    let territory = territory_of(state, board, id).ok_or(RuleError::NotOwned(id))?;
    let capital = territory.capital.ok_or(RuleError::NoCapital(id))?;
    Ok((territory, capital))
}

fn pay(state: &mut GameState, capital: TileId, cost: i64) -> Result<Event, RuleError> {
    let treasury = state.treasuries.entry(capital).or_default();
    if treasury.gold < cost {
        return Err(RuleError::CannotAfford {
            needed: cost,
            available: treasury.gold,
        });
    }
    treasury.gold -= cost;
    Ok(Event::TreasuryChanged {
        capital,
        gold: -cost,
        wheat: 0,
    })
}

/// R-BLD-01 / R-BLD-02: a town or field on an owned empty tile, paid by the territory's capital.
fn build(
    state: &mut GameState,
    board: &dyn Board,
    at: TileId,
    cover: Cover,
) -> Result<Vec<Event>, RuleError> {
    let (territory, capital) = own_territory(state, board, at)?;
    if state.tile(at).cover != Cover::None {
        return Err(RuleError::NotEmpty(at));
    }
    let existing = territory.count_cover(state, cover) as i64;
    let base = if cover == Cover::Town {
        TOWN_BASE_COST
    } else {
        FIELD_BASE_COST
    };
    let cost = base + existing;
    let available = state.treasuries.get(&capital).map_or(0, |t| t.gold);
    if available < cost {
        return Err(RuleError::CannotAfford {
            needed: cost,
            available,
        });
    }
    let paid = pay(state, capital, cost)?;
    state.tile_mut(at).cover = cover;
    Ok(vec![
        paid,
        Event::CoverChanged {
            tile: at,
            from: Cover::None,
            to: cover,
        },
    ])
}

/// R-UNIT-01: a pawn on an owned free land tile; cost 1 + units already in the territory.
fn recruit(state: &mut GameState, board: &dyn Board, at: TileId) -> Result<Vec<Event>, RuleError> {
    let (territory, capital) = own_territory(state, board, at)?;
    if state.tile(at).unit.is_some() {
        return Err(RuleError::Occupied(at));
    }
    let cost = PAWN_BASE_COST + territory.count_units(state, |_| true) as i64;
    let available = state.treasuries.get(&capital).map_or(0, |t| t.gold);
    if available < cost {
        return Err(RuleError::CannotAfford {
            needed: cost,
            available,
        });
    }
    let paid = pay(state, capital, cost)?;
    let born = state.turn;
    let owner = state.current;
    state.tile_mut(at).unit = Some(Unit {
        kind: UnitKind::Pawn,
        owner,
        born,
        acted: false,
    });
    Ok(vec![
        paid,
        Event::UnitCreated {
            tile: at,
            kind: UnitKind::Pawn,
        },
    ])
}

/// R-UNIT-08 / R-UNIT-09: one rank up; needs and consumes the unit's action.
fn upgrade(state: &mut GameState, board: &dyn Board, at: TileId) -> Result<Vec<Event>, RuleError> {
    let (territory, capital) = own_territory(state, board, at)?;
    let unit = state
        .tile(at)
        .unit
        .filter(|u| u.owner == state.current)
        .ok_or(RuleError::NoUnit(at))?;
    if unit.acted {
        return Err(RuleError::AlreadyActed(at));
    }
    let (next, cost) = match unit.kind {
        UnitKind::Pawn => {
            let veterans = territory.count_units(state, |k| k != UnitKind::Pawn) as i64;
            (UnitKind::Warrior, WARRIOR_UPGRADE_BASE_COST + veterans)
        }
        UnitKind::Warrior => {
            let knights = territory.count_units(state, |k| k == UnitKind::Knight) as i64;
            (
                UnitKind::Knight,
                KNIGHT_UPGRADE_BASE_COST + KNIGHT_UPGRADE_STEP * knights,
            )
        }
        UnitKind::Knight => return Err(RuleError::MaxRank(at)),
    };
    let available = state.treasuries.get(&capital).map_or(0, |t| t.gold);
    if available < cost {
        return Err(RuleError::CannotAfford {
            needed: cost,
            available,
        });
    }
    let paid = pay(state, capital, cost)?;
    let u = state.tile_mut(at).unit.as_mut().expect("checked above");
    u.kind = next;
    u.acted = true;
    Ok(vec![
        paid,
        Event::UnitUpgraded {
            tile: at,
            kind: next,
        },
    ])
}

/// R-TURN-02: income for the faction that ends its turn (R-ECO-01..03), then the next living
/// faction moves; a full round increments `turn`. Town feeding, upkeep, forest growth and the
/// victory check are backlog rules (R-ECO-04, R-UNIT-02/03, R-FOR-03, R-VIC-*).
fn end_turn(state: &mut GameState, board: &dyn Board) -> Vec<Event> {
    let mut events = Vec::new();
    let ending = state.current;
    for territory in territories(state, board)
        .into_iter()
        .filter(|t| t.owner == ending)
    {
        let Some(capital) = territory.capital else {
            continue;
        };
        let mut income = Treasury::default();
        for &id in &territory.tiles {
            match state.tile(id).cover {
                Cover::None => income.wheat += EMPTY_WHEAT,
                Cover::Capital => {
                    income.wheat += CAPITAL_WHEAT;
                    income.gold += CAPITAL_GOLD;
                }
                Cover::Field => income.wheat += FIELD_WHEAT,
                Cover::Forest | Cover::Town => {}
            }
        }
        let treasury = state.treasuries.entry(capital).or_default();
        treasury.gold += income.gold;
        treasury.wheat += income.wheat;
        *state.stats.all_time_gold.entry(ending).or_default() += income.gold;
        events.push(Event::TreasuryChanged {
            capital,
            gold: income.gold,
            wheat: income.wheat,
        });
    }

    let living = state.living_factions();
    let order = if living.is_empty() {
        state.settings.factions.clone()
    } else {
        living
    };
    let pos = order.iter().position(|f| *f == ending).unwrap_or(0);
    let next = order[(pos + 1) % order.len()];
    if (pos + 1) % order.len() == 0 {
        state.turn += 1;
        state.stats.turns = state.turn;
    }
    state.current = next;
    for tile in &mut state.tiles {
        if let Some(u) = tile.unit.as_mut()
            && u.owner == next
        {
            u.acted = false;
        }
    }
    events.push(Event::TurnStarted {
        faction: next,
        turn: state.turn,
    });
    events
}
