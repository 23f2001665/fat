//! Slint UI controller.
//!
//! The UI layer only formats values and routes user intent. Validation,
//! calculations, and persistence remain in the service/repository layers.

mod charts;
mod format;

slint::include_modules!();

use std::cell::RefCell;
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

use chrono::NaiveDate;
use semver::Version;
use slint::{Image, ModelRc, SharedString, VecModel};

use crate::database::Database;
use crate::error::{AppError, AppResult};
use crate::models::analytics::{AnalyticsChart, AnalyticsReport, MuscleGroupStats};
use crate::models::body_measurement::{MeasurementMetric, MetricComparison, NewBodyMeasurement};
use crate::models::equipment::NO_EQUIPMENT_ID;
use crate::models::exercise::{ExerciseCategoryInput, MuscleAssignment, MuscleRole};
use crate::models::update::{UpdateAvailability, UpdateCheck};
use crate::models::workout::WorkoutSet;
use crate::services::workout_service::{EditSetInput, NewSetInput};
use crate::services::{
    analytics_service, library_service, profile_service, update_service::UpdateService,
    workout_service,
};
use crate::utils::calculations::bmi_category;
use crate::utils::date::{
    display_date, format_date, parse_date_field, parse_optional_date_field, today_local, DateRange,
    Period,
};

const DB_FILE_NAME: &str = "workout.db";

pub fn run() -> Result<(), slint::PlatformError> {
    run_with_data_dir(resolve_data_dir())
}

/// Runs the UI with an explicit data directory.
///
/// Android supplies its app-private directory after the activity has been
/// initialized, while desktop uses [`resolve_data_dir`].
pub fn run_with_data_dir(data_dir: PathBuf) -> Result<(), slint::PlatformError> {
    let ui = Rc::new(AppWindow::new()?);
    match AppController::create(Rc::clone(&ui), data_dir) {
        Ok(controller) => {
            controller.initialize();
            ui.run()
        }
        Err(error) => {
            ui.set_fatal_error(error.user_message().into());
            ui.run()
        }
    }
}

/// Data directory selection:
/// 1. `FAT_DATA_DIR`, when set and non-empty,
/// 2. the platform local data directory + `fat-workout`.
pub fn resolve_data_dir() -> PathBuf {
    if let Some(value) = std::env::var_os("FAT_DATA_DIR") {
        let path = PathBuf::from(value);
        if !path.as_os_str().is_empty() {
            return path;
        }
    }
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("fat-workout")
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LibraryTab {
    Exercises,
    Muscles,
    Equipment,
}

impl LibraryTab {
    fn index(self) -> i32 {
        match self {
            Self::Exercises => 0,
            Self::Muscles => 1,
            Self::Equipment => 2,
        }
    }

    fn from_index(value: i32) -> Self {
        match value {
            1 => Self::Muscles,
            2 => Self::Equipment,
            _ => Self::Exercises,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PickerMode {
    SetExercise,
    SetEquipment,
    AnalyticsExercise,
    AnalyticsMuscle,
    ExerciseMuscle(MuscleRole),
}

#[derive(Clone)]
struct AssignmentEdit {
    muscle_id: i64,
    name: String,
    role: MuscleRole,
}

enum ConfirmAction {
    DeleteSet { date: NaiveDate, sequence: i64 },
}

struct UiState {
    workout_date: NaiveDate,
    workout_sets: Vec<WorkoutSet>,
    set_editing: Option<i64>,
    set_exercise_id: i64,
    set_equipment_id: i64,

    picker_mode: Option<PickerMode>,
    picker_search: String,
    pending_muscle_role: MuscleRole,
    pending_confirm: Option<ConfirmAction>,

    analytics_period: Period,
    analytics_exercise: Option<i64>,
    analytics_muscle: Option<i64>,
    analytics_chart: AnalyticsChart,
    analytics_report: Option<AnalyticsReport>,

    library_tab: LibraryTab,
    library_search: String,
    exercise_editor_id: Option<i64>,
    exercise_editor_assignments: Vec<AssignmentEdit>,
    muscle_editor_id: Option<i64>,
    equipment_editor_id: Option<i64>,

    update_rx: Option<Receiver<UpdateEvent>>,
    update_running: bool,
    update_message: String,
    update_rollback_available: bool,
}

impl UiState {
    fn new(today: NaiveDate) -> Self {
        Self {
            workout_date: today,
            workout_sets: Vec::new(),
            set_editing: None,
            set_exercise_id: 0,
            set_equipment_id: NO_EQUIPMENT_ID,
            picker_mode: None,
            picker_search: String::new(),
            pending_muscle_role: MuscleRole::Primary,
            pending_confirm: None,
            analytics_period: Period::Last30Days,
            analytics_exercise: None,
            analytics_muscle: None,
            analytics_chart: AnalyticsChart::WeightProgression,
            analytics_report: None,
            library_tab: LibraryTab::Exercises,
            library_search: String::new(),
            exercise_editor_id: None,
            exercise_editor_assignments: Vec::new(),
            muscle_editor_id: None,
            equipment_editor_id: None,
            update_rx: None,
            update_running: false,
            update_message: String::new(),
            update_rollback_available: false,
        }
    }
}

#[derive(Debug)]
enum UpdateEvent {
    Progress {
        downloaded: u64,
        total: Option<u64>,
    },
    Finished {
        message: String,
        rollback_available: Option<bool>,
    },
    Failed(String),
}

#[derive(Clone, Copy)]
enum UpdateTask {
    Check,
    Download,
    Install,
    DownloadRollback,
    InstallRollback,
}

impl UpdateTask {
    fn action_label(self) -> &'static str {
        match self {
            Self::Check => "Checking for updates...",
            Self::Download => "Downloading and verifying update...",
            Self::Install => "Backing up the database and preparing installation...",
            Self::DownloadRollback => "Downloading rollback package...",
            Self::InstallRollback => "Preparing rollback installation...",
        }
    }
}

struct AppController {
    ui: Rc<AppWindow>,
    data_dir: PathBuf,
    db: RefCell<Database>,
    schema_version: i64,
    state: RefCell<UiState>,
}

impl AppController {
    fn create(ui: Rc<AppWindow>, data_dir: PathBuf) -> AppResult<Rc<Self>> {
        std::fs::create_dir_all(&data_dir)?;
        let db_path = data_dir.join(DB_FILE_NAME);
        let db = Database::open(&db_path)?;
        let schema_version = db.schema_version()?;
        // Ensures the update configuration file exists on first launch.
        UpdateService::new(&data_dir, &db_path, schema_version)?;

        Ok(Rc::new(Self {
            ui,
            data_dir,
            db: RefCell::new(db),
            schema_version,
            state: RefCell::new(UiState::new(today_local())),
        }))
    }

    fn db_path(&self) -> PathBuf {
        self.data_dir.join(DB_FILE_NAME)
    }

    fn initialize(self: &Rc<Self>) {
        self.wire_callbacks();
        self.ui.set_workout_date(format_date(today_local()).into());
        self.refresh_all();
    }

    // ---------------------------------------------------------------------
    // Callback wiring
    // ---------------------------------------------------------------------

    fn wire_callbacks(self: &Rc<Self>) {
        let ui = Rc::clone(&self.ui);
        let weak = Rc::downgrade(self);

        macro_rules! connect0 {
            ($setter:ident, $method:ident) => {{
                let weak = weak.clone();
                ui.$setter(move || {
                    if let Some(controller) = weak.upgrade() {
                        controller.$method();
                    }
                });
            }};
        }
        macro_rules! connect1 {
            ($setter:ident, $method:ident) => {{
                let weak = weak.clone();
                ui.$setter(move |value| {
                    if let Some(controller) = weak.upgrade() {
                        controller.$method(value);
                    }
                });
            }};
        }

        connect0!(on_set_workout_date, set_workout_date);
        connect0!(on_workout_today, workout_today);
        connect0!(on_open_add_set, open_add_set);
        connect0!(on_save_set, save_set);
        connect0!(on_cancel_set_form, cancel_set_form);
        connect0!(on_pick_set_exercise, pick_set_exercise);
        connect0!(on_pick_set_equipment, pick_set_equipment);
        connect0!(on_open_custom_period, open_custom_period);
        connect0!(on_apply_custom_period, apply_custom_period);
        connect0!(on_cancel_custom_period, cancel_custom_period);
        connect0!(on_pick_analytics_exercise, pick_analytics_exercise);
        connect0!(on_pick_analytics_muscle, pick_analytics_muscle);
        connect0!(on_open_add_library, open_add_library);
        connect0!(on_save_exercise_editor, save_exercise_editor);
        connect0!(on_cancel_exercise_editor, cancel_exercise_editor);
        connect0!(on_save_muscle_editor, save_muscle_editor);
        connect0!(on_cancel_muscle_editor, cancel_muscle_editor);
        connect0!(on_save_equipment_editor, save_equipment_editor);
        connect0!(on_cancel_equipment_editor, cancel_equipment_editor);
        connect0!(on_open_profile_edit, open_profile_edit);
        connect0!(on_save_profile, save_profile);
        connect0!(on_cancel_profile_edit, cancel_profile_edit);
        connect0!(on_open_measurement_form, open_measurement_form);
        connect0!(on_save_measurement, save_measurement);
        connect0!(on_cancel_measurement_form, cancel_measurement_form);
        connect0!(on_check_updates, check_updates);
        connect0!(on_download_update, download_update);
        connect0!(on_install_update, install_update);
        connect0!(on_download_rollback, download_rollback);
        connect0!(on_install_rollback, install_rollback);
        connect0!(on_poll_updates, poll_updates);
        connect0!(on_picker_cancel, picker_cancel);
        connect0!(on_confirm_yes, confirm_yes);
        connect0!(on_confirm_no, confirm_no);

        connect1!(on_tab_changed, tab_changed);
        connect1!(on_open_edit_set, open_edit_set);
        connect1!(on_request_delete_set, request_delete_set);
        connect1!(on_select_history_day, select_history_day);
        connect1!(on_select_period, select_period);
        connect1!(on_select_chart, select_chart);
        connect1!(on_library_search_edited, library_search_edited);
        connect1!(on_open_edit_library, open_edit_library);
        connect1!(on_toggle_library_active, toggle_library_active);
        connect1!(on_select_library_tab, select_library_tab);
        connect1!(on_set_exercise_category, set_exercise_category);
        connect1!(on_add_exercise_muscle, add_exercise_muscle);
        connect1!(on_toggle_exercise_muscle, toggle_exercise_muscle);
        connect1!(on_remove_exercise_muscle, remove_exercise_muscle);
        connect1!(on_picker_search_edited, picker_search_edited);
        connect1!(on_picker_select, picker_select);
        connect1!(on_picker_create, picker_create);
    }

    // ---------------------------------------------------------------------
    // Shared helpers
    // ---------------------------------------------------------------------

    fn toast(&self, message: impl Into<String>) {
        self.ui.set_status_message(message.into().into());
        self.ui.set_status_error(false);
    }

    fn toast_error(&self, message: impl Into<String>) {
        self.ui.set_status_message(message.into().into());
        self.ui.set_status_error(true);
    }

    fn report_error(&self, error: &AppError) {
        self.toast_error(error.user_message());
    }

    fn refresh_all(&self) {
        self.refresh_workout();
        self.refresh_history();
        self.refresh_library();
        self.refresh_analytics();
        self.refresh_profile();
        self.refresh_update_view();
    }

    // ---------------------------------------------------------------------
    // Workout
    // ---------------------------------------------------------------------

    fn refresh_workout(&self) {
        let date = self.state.borrow().workout_date;
        let day = match workout_service::workout_for_date(self.db.borrow().connection(), date) {
            Ok(day) => day,
            Err(error) => {
                self.report_error(&error);
                return;
            }
        };

        let rows: Vec<WorkoutSetRow> = day
            .sets
            .iter()
            .map(|set| {
                let reps = format::reps(set.complete_reps, set.partial_reps);
                let mut detail = format!("{} kg × {} reps", format::number(set.weight, 2), reps);
                if let Some(rir) = set.rir {
                    detail.push_str(&format!("  •  RIR {rir}"));
                }
                if let Some(rest) = set.rest_seconds {
                    detail.push_str(&format!("  •  Rest {}s", rest));
                }
                let extra = set.extra_info.clone().unwrap_or_default();
                WorkoutSetRow {
                    sequence: set.sequence_number as i32,
                    title: set.exercise_name.clone().into(),
                    detail: detail.into(),
                    equipment: set.equipment_name.clone().into(),
                    volume: format::kg(set.volume()).into(),
                    has_extra: !extra.trim().is_empty(),
                    extra: extra.into(),
                }
            })
            .collect();

        let set_count = day.sets.len();
        let exercise_count = day.exercise_names().len();
        let summary = if set_count == 0 {
            "No sets logged".to_string()
        } else {
            format!(
                "{} {}  •  {} {}  •  {} total volume",
                set_count,
                if set_count == 1 { "set" } else { "sets" },
                exercise_count,
                if exercise_count == 1 {
                    "exercise"
                } else {
                    "exercises"
                },
                format::kg(day.total_volume())
            )
        };

        self.ui
            .set_workout_date_label(day.date.format("%A, %d %B %Y").to_string().into());
        self.ui.set_workout_summary(summary.into());
        self.ui.set_workout_empty(set_count == 0);
        self.ui.set_workout_sets(ModelRc::new(VecModel::from(rows)));
        self.state.borrow_mut().workout_sets = day.sets;
    }

    fn set_workout_date(&self) {
        let value = self.ui.get_workout_date().to_string();
        let date = match parse_date_field(&value, "Workout date") {
            Ok(date) => date,
            Err(error) => {
                self.toast_error(error.user_message());
                return;
            }
        };
        if date > today_local() {
            self.toast_error("Workouts cannot be logged for a future date");
            return;
        }
        if date < NaiveDate::from_ymd_opt(1900, 1, 1).unwrap() {
            self.toast_error("Workout date is too far in the past");
            return;
        }
        self.state.borrow_mut().workout_date = date;
        self.refresh_workout();
    }

    fn workout_today(&self) {
        let today = today_local();
        self.state.borrow_mut().workout_date = today;
        self.ui.set_workout_date(format_date(today).into());
        self.refresh_workout();
    }

    fn open_add_set(&self) {
        let date = self.state.borrow().workout_date;
        if date > today_local() {
            self.toast_error("Workouts cannot be logged for a future date");
            return;
        }
        let conn = self.db.borrow();
        let exercises = match library_service::list_exercises(conn.connection(), false, None) {
            Ok(exercises) => exercises,
            Err(error) => {
                self.report_error(&error);
                return;
            }
        };
        let equipment = match library_service::list_equipment(conn.connection(), false, None) {
            Ok(equipment) => equipment,
            Err(error) => {
                self.report_error(&error);
                return;
            }
        };
        drop(conn);

        let mut state = self.state.borrow_mut();
        if !exercises
            .iter()
            .any(|exercise| exercise.id == state.set_exercise_id)
        {
            state.set_exercise_id = exercises.first().map(|exercise| exercise.id).unwrap_or(0);
        }
        if !equipment
            .iter()
            .any(|item| item.id == state.set_equipment_id)
        {
            state.set_equipment_id = equipment
                .first()
                .map(|item| item.id)
                .unwrap_or(NO_EQUIPMENT_ID);
        }
        state.set_editing = None;
        let exercise_name = exercises
            .iter()
            .find(|exercise| exercise.id == state.set_exercise_id)
            .map(|exercise| exercise.name.clone())
            .unwrap_or_else(|| "Choose exercise".into());
        let equipment_name = equipment
            .iter()
            .find(|item| item.id == state.set_equipment_id)
            .map(|item| item.name.clone())
            .unwrap_or_else(|| "Choose equipment".into());
        drop(state);

        self.ui.set_set_form_title("Add set".into());
        self.ui.set_set_form_editing(false);
        self.ui.set_set_exercise_name(exercise_name.into());
        self.ui.set_set_equipment_name(equipment_name.into());
        self.ui.set_set_weight("0".into());
        self.ui.set_set_complete_reps("".into());
        self.ui.set_set_partial_reps("0".into());
        self.ui.set_set_rest("".into());
        self.ui.set_set_rir("".into());
        self.ui.set_set_extra("".into());
        self.ui.set_set_form_error("".into());
        self.ui.set_set_form_visible(true);
    }

    fn open_edit_set(&self, sequence: i32) {
        let sequence = sequence as i64;
        let set = {
            let state = self.state.borrow();
            state
                .workout_sets
                .iter()
                .find(|set| set.sequence_number == sequence)
                .cloned()
        };
        let Some(set) = set else {
            self.toast_error("That set is no longer present. Refresh the workout.");
            return;
        };

        {
            let mut state = self.state.borrow_mut();
            state.set_editing = Some(sequence);
            state.set_exercise_id = set.exercise_id;
            state.set_equipment_id = set.equipment_id;
        }

        self.ui
            .set_set_form_title(format!("Edit set #{sequence}").into());
        self.ui.set_set_form_editing(true);
        self.ui
            .set_set_exercise_name(set.exercise_name.clone().into());
        self.ui
            .set_set_equipment_name(set.equipment_name.clone().into());
        self.ui.set_set_weight(format::number(set.weight, 2).into());
        self.ui
            .set_set_complete_reps(set.complete_reps.to_string().into());
        self.ui
            .set_set_partial_reps(set.partial_reps.to_string().into());
        self.ui.set_set_rest(
            set.rest_seconds
                .map(|value| value.to_string())
                .unwrap_or_default()
                .into(),
        );
        self.ui.set_set_rir(
            set.rir
                .map(|value| value.to_string())
                .unwrap_or_default()
                .into(),
        );
        self.ui
            .set_set_extra(set.extra_info.unwrap_or_default().into());
        self.ui.set_set_form_error("".into());
        self.ui.set_set_form_visible(true);
    }

    fn save_set(&self) {
        let weight = match parse_optional_f64(&self.ui.get_set_weight(), "Weight") {
            Ok(Some(value)) => value,
            Ok(None) => {
                self.ui
                    .set_set_form_error("Weight is required (use 0 for bodyweight)".into());
                return;
            }
            Err(error) => {
                self.ui.set_set_form_error(error.user_message().into());
                return;
            }
        };
        let complete_reps =
            match parse_optional_i64(&self.ui.get_set_complete_reps(), "Complete reps") {
                Ok(Some(value)) => value,
                Ok(None) => 0,
                Err(error) => {
                    self.ui.set_set_form_error(error.user_message().into());
                    return;
                }
            };
        let partial_reps = match parse_optional_i64(&self.ui.get_set_partial_reps(), "Partial reps")
        {
            Ok(Some(value)) => value,
            Ok(None) => 0,
            Err(error) => {
                self.ui.set_set_form_error(error.user_message().into());
                return;
            }
        };
        let rest_seconds = match parse_optional_i64(&self.ui.get_set_rest(), "Rest") {
            Ok(value) => value,
            Err(error) => {
                self.ui.set_set_form_error(error.user_message().into());
                return;
            }
        };
        let rir = match parse_optional_i64(&self.ui.get_set_rir(), "RIR") {
            Ok(value) => value,
            Err(error) => {
                self.ui.set_set_form_error(error.user_message().into());
                return;
            }
        };
        let extra_info = self.ui.get_set_extra().to_string();

        let (date, editing, exercise_id, equipment_id) = {
            let state = self.state.borrow();
            (
                state.workout_date,
                state.set_editing,
                state.set_exercise_id,
                state.set_equipment_id,
            )
        };
        if exercise_id <= 0 {
            self.ui
                .set_set_form_error("Choose an exercise before saving the set".into());
            return;
        }

        let result = if let Some(sequence) = editing {
            let input = EditSetInput {
                exercise_id,
                equipment_id,
                weight,
                complete_reps,
                partial_reps,
                rest_seconds,
                rir,
                extra_info: extra_info.clone(),
            };
            workout_service::edit_set(self.db.borrow().connection(), date, sequence, &input)
                .map(|_| (sequence, false))
        } else {
            let input = NewSetInput {
                date,
                exercise_id,
                equipment_id,
                weight,
                complete_reps,
                partial_reps,
                rest_seconds,
                rir,
                extra_info,
            };
            workout_service::add_set(self.db.borrow_mut().connection_mut(), &input)
                .map(|sequence| (sequence, true))
        };

        match result {
            Ok((sequence, added)) => {
                self.ui.set_set_form_visible(false);
                self.ui.set_set_form_error("".into());
                self.refresh_workout();
                self.refresh_history();
                self.refresh_analytics();
                if added {
                    self.toast(format!("Set #{sequence} added"));
                } else {
                    self.toast(format!("Set #{sequence} updated"));
                }
            }
            Err(error) => {
                self.ui.set_set_form_error(error.user_message().into());
            }
        }
    }

    fn cancel_set_form(&self) {
        self.ui.set_set_form_visible(false);
        self.ui.set_set_form_error("".into());
        self.state.borrow_mut().set_editing = None;
    }

    fn pick_set_exercise(&self) {
        self.open_picker(PickerMode::SetExercise);
    }

    fn pick_set_equipment(&self) {
        self.open_picker(PickerMode::SetEquipment);
    }

    fn request_delete_set(&self, sequence: i32) {
        let date = self.state.borrow().workout_date;
        let sequence = sequence as i64;
        {
            let mut state = self.state.borrow_mut();
            state.pending_confirm = Some(ConfirmAction::DeleteSet { date, sequence });
        }
        self.ui.set_confirm_title("Delete this set?".into());
        self.ui.set_confirm_message(format!(
            "Set #{sequence} will be removed. The remaining sets keep their original sequence numbers; they are never renumbered."
        ).into());
        self.ui.set_confirm_label("Delete set".into());
        self.ui.set_confirm_visible(true);
    }

    fn confirm_yes(&self) {
        let action = self.state.borrow_mut().pending_confirm.take();
        self.ui.set_confirm_visible(false);
        let Some(action) = action else {
            return;
        };
        match action {
            ConfirmAction::DeleteSet { date, sequence } => {
                match workout_service::delete_set(self.db.borrow().connection(), date, sequence) {
                    Ok(()) => {
                        self.refresh_workout();
                        self.refresh_history();
                        self.refresh_analytics();
                        self.toast(format!("Set #{sequence} deleted"));
                    }
                    Err(error) => self.report_error(&error),
                }
            }
        }
    }

    fn confirm_no(&self) {
        self.state.borrow_mut().pending_confirm = None;
        self.ui.set_confirm_visible(false);
    }

    // ---------------------------------------------------------------------
    // History
    // ---------------------------------------------------------------------

    fn refresh_history(&self) {
        let history = match workout_service::history(self.db.borrow().connection()) {
            Ok(history) => history,
            Err(error) => {
                self.report_error(&error);
                return;
            }
        };
        let rows: Vec<HistoryRow> = history
            .iter()
            .map(|day| HistoryRow {
                date_key: format_date(day.date).into(),
                date_label: display_date(day.date).into(),
                title: format!(
                    "{} {}",
                    day.set_count,
                    if day.set_count == 1 { "set" } else { "sets" }
                )
                .into(),
                detail: if day.exercise_names.is_empty() {
                    "No exercises".into()
                } else {
                    day.exercise_names.join("  •  ").into()
                },
                volume: format::kg(day.total_volume).into(),
                set_count: day.set_count as i32,
            })
            .collect();
        self.ui.set_history_empty(rows.is_empty());
        self.ui.set_history_rows(ModelRc::new(VecModel::from(rows)));
    }

    fn select_history_day(&self, date: SharedString) {
        let date = match parse_date_field(date.as_str(), "Workout date") {
            Ok(date) => date,
            Err(error) => {
                self.report_error(&error);
                return;
            }
        };
        self.state.borrow_mut().workout_date = date;
        self.ui.set_workout_date(format_date(date).into());
        self.ui.set_active_tab(0);
        self.refresh_workout();
    }

    fn tab_changed(&self, tab: i32) {
        match tab {
            0 => self.refresh_workout(),
            1 => self.refresh_history(),
            2 => self.refresh_analytics(),
            3 => self.refresh_library(),
            4 => {
                self.refresh_profile();
                self.refresh_update_view();
            }
            _ => {}
        }
    }

    // ---------------------------------------------------------------------
    // Analytics
    // ---------------------------------------------------------------------

    fn refresh_analytics(&self) {
        let (period, exercise_id, muscle_id, chart) = {
            let state = self.state.borrow();
            (
                state.analytics_period.clone(),
                state.analytics_exercise,
                state.analytics_muscle,
                state.analytics_chart,
            )
        };
        let report = match analytics_service::build_report_for_today(
            self.db.borrow().connection(),
            period,
            exercise_id,
            muscle_id,
        ) {
            Ok(report) => report,
            Err(error) => {
                self.report_error(&error);
                return;
            }
        };

        self.ui.set_analytics_period_key(report.period.key().into());
        self.ui
            .set_analytics_period_label(report.period.label().into());
        self.ui
            .set_analytics_range(range_label(report.range).into());
        self.ui.set_analytics_exercise(
            format!(
                "Exercise: {}",
                report
                    .exercise
                    .exercise_name
                    .clone()
                    .unwrap_or_else(|| "any".into())
            )
            .into(),
        );
        let muscle_label = match muscle_id {
            Some(id) => self
                .db
                .borrow()
                .connection()
                .query_row(
                    "SELECT name FROM muscle_groups WHERE id = ?1",
                    [id],
                    |row| row.get::<_, String>(0),
                )
                .ok()
                .unwrap_or_else(|| format!("#{id}")),
            None => "any".into(),
        };
        self.ui
            .set_analytics_muscle(format!("Muscle: {muscle_label}").into());

        let exercise = &report.exercise;
        self.ui
            .set_analytics_total_sets(exercise.total_sets.to_string().into());
        self.ui
            .set_analytics_total_reps(format::number(exercise.total_effective_reps, 1).into());
        self.ui
            .set_analytics_total_volume(format::kg(exercise.total_volume).into());
        self.ui.set_analytics_max_weight(
            exercise
                .max_weight
                .map(format::kg)
                .unwrap_or_else(|| "-".into())
                .into(),
        );
        self.ui.set_analytics_average_weight(
            exercise
                .average_weight
                .map(format::kg)
                .unwrap_or_else(|| "-".into())
                .into(),
        );
        self.ui.set_analytics_average_rir(
            exercise
                .average_rir
                .map(|value| format::number(value, 1))
                .unwrap_or_else(|| "-".into())
                .into(),
        );
        self.ui.set_analytics_has_data(report.has_data);
        self.ui.set_analytics_chart(chart_key(chart).into());

        let muscle_rows: Vec<MuscleStatRow> =
            report.muscle_groups.iter().map(muscle_stat_row).collect();
        self.ui.set_analytics_muscles_empty(muscle_rows.is_empty());
        self.ui
            .set_analytics_muscles(ModelRc::new(VecModel::from(muscle_rows)));

        self.state.borrow_mut().analytics_report = Some(report);
        self.update_analytics_chart();
    }

    fn update_analytics_chart(&self) {
        let (report, chart) = {
            let state = self.state.borrow();
            (state.analytics_report.clone(), state.analytics_chart)
        };
        let Some(report) = report else {
            return;
        };

        let image: Option<Image>;
        let mut caption_start = String::new();
        let mut caption_end = String::new();
        let mut bars: Vec<NamedValueRow> = Vec::new();
        let mut bars_title = String::new();

        match chart {
            AnalyticsChart::WeightProgression => {
                let points = &report.exercise.weight_progression;
                image = charts::render_line(points);
                if let (Some(first), Some(last)) = (points.first(), points.last()) {
                    caption_start = display_date(first.date);
                    caption_end = display_date(last.date);
                }
                if let Some(bounds) = charts::series_bounds(points) {
                    bars_title = format!(
                        "Range: {} – {} kg  •  {} actual observations",
                        format::number(bounds.min, 1),
                        format::number(bounds.max, 1),
                        points.len()
                    );
                }
            }
            AnalyticsChart::VolumeProgression => {
                let points = &report.exercise.volume_progression;
                image = charts::render_line(points);
                if let (Some(first), Some(last)) = (points.first(), points.last()) {
                    caption_start = display_date(first.date);
                    caption_end = display_date(last.date);
                }
                if let Some(bounds) = charts::series_bounds(points) {
                    bars_title = format!(
                        "Range: {} – {}  •  {} actual observations",
                        format::kg(bounds.min),
                        format::kg(bounds.max),
                        points.len()
                    );
                }
            }
            AnalyticsChart::WeeklyVolume => {
                let points = &report.weekly_volume;
                let values: Vec<f64> = points.iter().map(|point| point.value).collect();
                image = charts::render_bars(&values);
                if let (Some(first), Some(last)) = (points.first(), points.last()) {
                    caption_start = format!("Week of {}", display_date(first.date));
                    caption_end = format!("Week of {}", display_date(last.date));
                }
                bars_title = "Weekly volume (weeks with recorded sets)".into();
                bars = points
                    .iter()
                    .rev()
                    .take(10)
                    .map(|point| NamedValueRow {
                        label: format!("Week of {}", display_date(point.date)).into(),
                        value: format::kg(point.value).into(),
                    })
                    .collect();
            }
            AnalyticsChart::MuscleDistribution => {
                let values: Vec<f64> = report
                    .muscle_distribution
                    .iter()
                    .map(|row| row.value)
                    .collect();
                image = charts::render_bars(&values);
                bars_title = "Attributed volume by muscle group".into();
                bars = report
                    .muscle_distribution
                    .iter()
                    .take(10)
                    .map(|row| NamedValueRow {
                        label: row.label.clone().into(),
                        value: format::kg(row.value).into(),
                    })
                    .collect();
            }
            AnalyticsChart::ExerciseFrequency => {
                let values: Vec<f64> = report
                    .exercise_frequency
                    .iter()
                    .map(|row| row.value)
                    .collect();
                image = charts::render_bars(&values);
                bars_title = "Sets per exercise".into();
                bars = report
                    .exercise_frequency
                    .iter()
                    .take(10)
                    .map(|row| NamedValueRow {
                        label: row.label.clone().into(),
                        value: format!("{} sets", format::number(row.value, 0)).into(),
                    })
                    .collect();
            }
        }

        let empty = image.is_none();
        self.ui.set_chart_empty(empty);
        self.ui.set_chart_image(image.unwrap_or_default());
        self.ui.set_analytics_chart_title(chart.title().into());
        self.ui
            .set_analytics_chart_caption_start(caption_start.into());
        self.ui.set_analytics_chart_caption_end(caption_end.into());
        self.ui.set_chart_bars_title(bars_title.into());
        self.ui.set_chart_bars(ModelRc::new(VecModel::from(bars)));
    }

    fn select_period(&self, key: SharedString) {
        let key = key.to_string();
        if key == "custom" {
            self.open_custom_period();
            return;
        }
        let period = match Period::from_key(&key) {
            Ok(period) => period,
            Err(error) => {
                self.report_error(&error);
                return;
            }
        };
        self.state.borrow_mut().analytics_period = period;
        self.refresh_analytics();
    }

    fn open_custom_period(&self) {
        let today = today_local();
        let current = {
            let state = self.state.borrow();
            state.analytics_period.resolve(today).ok()
        };
        let (start, end) = match current {
            Some(range) => (range.start, range.end),
            None => (None, Some(today)),
        };
        self.ui
            .set_custom_period_start(start.map(format_date).unwrap_or_default().into());
        self.ui
            .set_custom_period_end(end.map(format_date).unwrap_or_default().into());
        self.ui.set_custom_period_error("".into());
        self.ui.set_custom_period_visible(true);
    }

    fn apply_custom_period(&self) {
        let start = match parse_date_field(&self.ui.get_custom_period_start(), "Start date") {
            Ok(date) => date,
            Err(error) => {
                self.ui.set_custom_period_error(error.user_message().into());
                return;
            }
        };
        let end = match parse_date_field(&self.ui.get_custom_period_end(), "End date") {
            Ok(date) => date,
            Err(error) => {
                self.ui.set_custom_period_error(error.user_message().into());
                return;
            }
        };
        let range = match DateRange::new(Some(start), Some(end)) {
            Ok(range) => range,
            Err(error) => {
                self.ui.set_custom_period_error(error.user_message().into());
                return;
            }
        };
        self.state.borrow_mut().analytics_period = Period::Custom(range);
        self.ui.set_custom_period_visible(false);
        self.ui.set_custom_period_error("".into());
        self.refresh_analytics();
    }

    fn cancel_custom_period(&self) {
        self.ui.set_custom_period_visible(false);
        self.ui.set_custom_period_error("".into());
    }

    fn select_chart(&self, key: SharedString) {
        let chart = AnalyticsChart::from_key(key.as_str());
        self.state.borrow_mut().analytics_chart = chart;
        self.ui.set_analytics_chart(chart_key(chart).into());
        self.update_analytics_chart();
    }

    fn pick_analytics_exercise(&self) {
        self.open_picker(PickerMode::AnalyticsExercise);
    }

    fn pick_analytics_muscle(&self) {
        self.open_picker(PickerMode::AnalyticsMuscle);
    }

    // ---------------------------------------------------------------------
    // Library
    // ---------------------------------------------------------------------

    fn refresh_library(&self) {
        let (tab, search) = {
            let state = self.state.borrow();
            (state.library_tab, state.library_search.clone())
        };
        let conn = self.db.borrow();
        let rows: Vec<LibraryRow> = match tab {
            LibraryTab::Exercises => {
                match library_service::list_exercises(conn.connection(), true, Some(&search)) {
                    Ok(exercises) => exercises
                        .into_iter()
                        .map(|exercise| {
                            let summary = exercise.muscle_summary();
                            LibraryRow {
                                id: exercise.id as i32,
                                title: exercise.name.into(),
                                subtitle: format!("{}  •  {}", exercise.category.as_str(), summary)
                                    .into(),
                                badge: if exercise.is_active {
                                    "Active".into()
                                } else {
                                    "Archived".into()
                                },
                                is_active: exercise.is_active,
                                can_toggle: true,
                                toggle_label: if exercise.is_active {
                                    "Archive".into()
                                } else {
                                    "Activate".into()
                                },
                            }
                        })
                        .collect(),
                    Err(error) => {
                        drop(conn);
                        self.report_error(&error);
                        return;
                    }
                }
            }
            LibraryTab::Muscles => {
                match library_service::list_muscle_groups(conn.connection(), true, Some(&search)) {
                    Ok(groups) => groups
                        .into_iter()
                        .map(|group| LibraryRow {
                            id: group.id as i32,
                            title: group.name.into(),
                            subtitle: group
                                .description
                                .unwrap_or_else(|| "No description".into())
                                .into(),
                            badge: if group.is_active {
                                "Active".into()
                            } else {
                                "Archived".into()
                            },
                            is_active: group.is_active,
                            can_toggle: true,
                            toggle_label: if group.is_active {
                                "Archive".into()
                            } else {
                                "Activate".into()
                            },
                        })
                        .collect(),
                    Err(error) => {
                        drop(conn);
                        self.report_error(&error);
                        return;
                    }
                }
            }
            LibraryTab::Equipment => {
                match library_service::list_equipment(conn.connection(), true, Some(&search)) {
                    Ok(items) => items
                        .into_iter()
                        .map(|item| {
                            let reserved = item.is_reserved();
                            LibraryRow {
                                id: item.id as i32,
                                title: item.name.into(),
                                subtitle: item
                                    .category
                                    .unwrap_or_else(|| "Uncategorized".into())
                                    .into(),
                                badge: if reserved {
                                    "Reserved".into()
                                } else if item.is_active {
                                    "Active".into()
                                } else {
                                    "Archived".into()
                                },
                                is_active: item.is_active,
                                can_toggle: !reserved,
                                toggle_label: if item.is_active {
                                    "Archive".into()
                                } else {
                                    "Activate".into()
                                },
                            }
                        })
                        .collect(),
                    Err(error) => {
                        drop(conn);
                        self.report_error(&error);
                        return;
                    }
                }
            }
        };
        drop(conn);
        self.ui.set_library_tab(tab.index());
        self.ui.set_library_empty(rows.is_empty());
        self.ui.set_library_rows(ModelRc::new(VecModel::from(rows)));
    }

    fn select_library_tab(&self, tab: i32) {
        let tab = LibraryTab::from_index(tab);
        {
            let mut state = self.state.borrow_mut();
            state.library_tab = tab;
            state.library_search.clear();
        }
        self.ui.set_library_tab(tab.index());
        self.ui.set_library_search("".into());
        self.refresh_library();
    }

    fn library_search_edited(&self, text: SharedString) {
        self.state.borrow_mut().library_search = text.to_string();
        self.refresh_library();
    }

    fn open_add_library(&self) {
        let tab = self.state.borrow().library_tab;
        match tab {
            LibraryTab::Exercises => {
                self.state.borrow_mut().exercise_editor_id = None;
                self.state.borrow_mut().exercise_editor_assignments.clear();
                self.ui.set_exercise_editor_title("New exercise".into());
                self.ui.set_exercise_editor_name("".into());
                self.ui.set_exercise_editor_description("".into());
                self.ui.set_exercise_editor_category("Other".into());
                self.ui.set_exercise_editor_active(true);
                self.refresh_assignment_rows();
                self.ui.set_exercise_editor_error("".into());
                self.ui.set_exercise_editor_visible(true);
            }
            LibraryTab::Muscles => {
                self.state.borrow_mut().muscle_editor_id = None;
                self.ui.set_muscle_editor_title("New muscle group".into());
                self.ui.set_muscle_editor_name("".into());
                self.ui.set_muscle_editor_description("".into());
                self.ui.set_muscle_editor_active(true);
                self.ui.set_muscle_editor_error("".into());
                self.ui.set_muscle_editor_visible(true);
            }
            LibraryTab::Equipment => {
                self.state.borrow_mut().equipment_editor_id = None;
                self.ui.set_equipment_editor_title("New equipment".into());
                self.ui.set_equipment_editor_name("".into());
                self.ui.set_equipment_editor_category("".into());
                self.ui.set_equipment_editor_active(true);
                self.ui.set_equipment_editor_reserved(false);
                self.ui.set_equipment_editor_error("".into());
                self.ui.set_equipment_editor_visible(true);
            }
        }
    }

    fn open_edit_library(&self, id: i32) {
        let id = id as i64;
        let tab = self.state.borrow().library_tab;
        match tab {
            LibraryTab::Exercises => {
                let exercise =
                    match library_service::get_exercise(self.db.borrow().connection(), id) {
                        Ok(exercise) => exercise,
                        Err(error) => {
                            self.report_error(&error);
                            return;
                        }
                    };
                {
                    let mut state = self.state.borrow_mut();
                    state.exercise_editor_id = Some(id);
                    state.exercise_editor_assignments = exercise
                        .muscles
                        .iter()
                        .map(|muscle| AssignmentEdit {
                            muscle_id: muscle.muscle_group_id,
                            name: muscle.muscle_group_name.clone(),
                            role: muscle.role,
                        })
                        .collect();
                }
                self.ui.set_exercise_editor_title("Edit exercise".into());
                self.ui.set_exercise_editor_name(exercise.name.into());
                self.ui.set_exercise_editor_description(
                    exercise.description.unwrap_or_default().into(),
                );
                self.ui
                    .set_exercise_editor_category(exercise.category.as_str().into());
                self.ui.set_exercise_editor_active(exercise.is_active);
                self.refresh_assignment_rows();
                self.ui.set_exercise_editor_error("".into());
                self.ui.set_exercise_editor_visible(true);
            }
            LibraryTab::Muscles => {
                let group =
                    match library_service::get_muscle_group(self.db.borrow().connection(), id) {
                        Ok(group) => group,
                        Err(error) => {
                            self.report_error(&error);
                            return;
                        }
                    };
                self.state.borrow_mut().muscle_editor_id = Some(id);
                self.ui.set_muscle_editor_title("Edit muscle group".into());
                self.ui.set_muscle_editor_name(group.name.into());
                self.ui
                    .set_muscle_editor_description(group.description.unwrap_or_default().into());
                self.ui.set_muscle_editor_active(group.is_active);
                self.ui.set_muscle_editor_error("".into());
                self.ui.set_muscle_editor_visible(true);
            }
            LibraryTab::Equipment => {
                let item = match library_service::get_equipment(self.db.borrow().connection(), id) {
                    Ok(item) => item,
                    Err(error) => {
                        self.report_error(&error);
                        return;
                    }
                };
                let reserved = item.is_reserved();
                self.state.borrow_mut().equipment_editor_id = Some(id);
                self.ui.set_equipment_editor_title("Edit equipment".into());
                self.ui.set_equipment_editor_name(item.name.into());
                self.ui
                    .set_equipment_editor_category(item.category.unwrap_or_default().into());
                self.ui.set_equipment_editor_active(item.is_active);
                self.ui.set_equipment_editor_reserved(reserved);
                self.ui.set_equipment_editor_error("".into());
                self.ui.set_equipment_editor_visible(true);
            }
        }
    }

    fn toggle_library_active(&self, id: i32) {
        let id = id as i64;
        let tab = self.state.borrow().library_tab;
        let result: AppResult<()> = (|| match tab {
            LibraryTab::Exercises => {
                let exercise = library_service::get_exercise(self.db.borrow().connection(), id)?;
                library_service::set_exercise_active(
                    self.db.borrow().connection(),
                    id,
                    !exercise.is_active,
                )
            }
            LibraryTab::Muscles => library_service::set_muscle_group_active(
                self.db.borrow().connection(),
                id,
                !library_service::get_muscle_group(self.db.borrow().connection(), id)?.is_active,
            ),
            LibraryTab::Equipment => {
                let item = library_service::get_equipment(self.db.borrow().connection(), id)?;
                library_service::set_equipment_active(
                    self.db.borrow().connection(),
                    id,
                    !item.is_active,
                )
            }
        })();
        match result {
            Ok(()) => {
                self.refresh_library();
                self.toast("Library record updated");
            }
            Err(error) => self.report_error(&error),
        }
    }

    fn refresh_assignment_rows(&self) {
        let rows: Vec<AssignmentRow> = self
            .state
            .borrow()
            .exercise_editor_assignments
            .iter()
            .map(|assignment| AssignmentRow {
                muscle_id: assignment.muscle_id as i32,
                name: assignment.name.clone().into(),
                role: assignment.role.as_str().into(),
                role_key: match assignment.role {
                    MuscleRole::Primary => "primary".into(),
                    MuscleRole::Secondary => "secondary".into(),
                },
            })
            .collect();
        self.ui
            .set_exercise_editor_assignments(ModelRc::new(VecModel::from(rows)));
    }

    fn save_exercise_editor(&self) {
        let name = self.ui.get_exercise_editor_name().to_string();
        let description = self.ui.get_exercise_editor_description().to_string();
        let category =
            ExerciseCategoryInput::from_ui(self.ui.get_exercise_editor_category().as_str());
        let active = self.ui.get_exercise_editor_active();
        let assignments: Vec<MuscleAssignment> = self
            .state
            .borrow()
            .exercise_editor_assignments
            .iter()
            .map(|assignment| MuscleAssignment {
                muscle_group_id: assignment.muscle_id,
                role: assignment.role,
            })
            .collect();
        let editor_id = self.state.borrow().exercise_editor_id;

        let result: AppResult<()> = match editor_id {
            Some(id) => library_service::update_exercise(
                self.db.borrow_mut().connection_mut(),
                id,
                &name,
                &description,
                category,
                active,
                &assignments,
            ),
            None => {
                match library_service::create_exercise(
                    self.db.borrow_mut().connection_mut(),
                    &name,
                    &description,
                    category,
                    &assignments,
                ) {
                    Ok(id) if !active => library_service::set_exercise_active(
                        self.db.borrow().connection(),
                        id,
                        false,
                    ),
                    Ok(_) => Ok(()),
                    Err(error) => Err(error),
                }
            }
        };

        match result {
            Ok(()) => {
                self.ui.set_exercise_editor_visible(false);
                self.ui.set_exercise_editor_error("".into());
                self.refresh_library();
                self.refresh_analytics();
                self.toast("Exercise saved");
            }
            Err(error) => {
                self.ui
                    .set_exercise_editor_error(error.user_message().into());
            }
        }
    }

    fn cancel_exercise_editor(&self) {
        self.ui.set_exercise_editor_visible(false);
        self.ui.set_exercise_editor_error("".into());
    }

    fn set_exercise_category(&self, value: SharedString) {
        self.ui.set_exercise_editor_category(value);
    }

    fn add_exercise_muscle(&self, role: SharedString) {
        let role = match role.as_str() {
            "secondary" => MuscleRole::Secondary,
            _ => MuscleRole::Primary,
        };
        self.state.borrow_mut().pending_muscle_role = role;
        self.open_picker(PickerMode::ExerciseMuscle(role));
    }

    fn toggle_exercise_muscle(&self, muscle_id: i32) {
        let muscle_id = muscle_id as i64;
        {
            let mut state = self.state.borrow_mut();
            if let Some(assignment) = state
                .exercise_editor_assignments
                .iter_mut()
                .find(|assignment| assignment.muscle_id == muscle_id)
            {
                assignment.role = match assignment.role {
                    MuscleRole::Primary => MuscleRole::Secondary,
                    MuscleRole::Secondary => MuscleRole::Primary,
                };
            }
        }
        self.refresh_assignment_rows();
    }

    fn remove_exercise_muscle(&self, muscle_id: i32) {
        let muscle_id = muscle_id as i64;
        self.state
            .borrow_mut()
            .exercise_editor_assignments
            .retain(|assignment| assignment.muscle_id != muscle_id);
        self.refresh_assignment_rows();
    }

    fn save_muscle_editor(&self) {
        let name = self.ui.get_muscle_editor_name().to_string();
        let description = self.ui.get_muscle_editor_description().to_string();
        let active = self.ui.get_muscle_editor_active();
        let editor_id = self.state.borrow().muscle_editor_id;
        let result = match editor_id {
            Some(id) => library_service::update_muscle_group(
                self.db.borrow().connection(),
                id,
                &name,
                &description,
                active,
            ),
            None => library_service::create_muscle_group(
                self.db.borrow().connection(),
                &name,
                &description,
            )
            .map(|_| ()),
        };
        match result {
            Ok(()) => {
                self.ui.set_muscle_editor_visible(false);
                self.ui.set_muscle_editor_error("".into());
                self.refresh_library();
                self.toast("Muscle group saved");
            }
            Err(error) => {
                self.ui.set_muscle_editor_error(error.user_message().into());
            }
        }
    }

    fn cancel_muscle_editor(&self) {
        self.ui.set_muscle_editor_visible(false);
        self.ui.set_muscle_editor_error("".into());
    }

    fn save_equipment_editor(&self) {
        let name = self.ui.get_equipment_editor_name().to_string();
        let category = self.ui.get_equipment_editor_category().to_string();
        let reserved = self.ui.get_equipment_editor_reserved();
        let active = if reserved {
            true
        } else {
            self.ui.get_equipment_editor_active()
        };
        let editor_id = self.state.borrow().equipment_editor_id;
        let result = match editor_id {
            Some(id) => library_service::update_equipment(
                self.db.borrow().connection(),
                id,
                &name,
                &category,
                active,
            ),
            None => {
                library_service::create_equipment(self.db.borrow().connection(), &name, &category)
                    .map(|_| ())
            }
        };
        match result {
            Ok(()) => {
                self.ui.set_equipment_editor_visible(false);
                self.ui.set_equipment_editor_error("".into());
                self.refresh_library();
                self.toast("Equipment saved");
            }
            Err(error) => {
                self.ui
                    .set_equipment_editor_error(error.user_message().into());
            }
        }
    }

    fn cancel_equipment_editor(&self) {
        self.ui.set_equipment_editor_visible(false);
        self.ui.set_equipment_editor_error("".into());
    }

    // ---------------------------------------------------------------------
    // Pickers
    // ---------------------------------------------------------------------

    fn open_picker(&self, mode: PickerMode) {
        {
            let mut state = self.state.borrow_mut();
            state.picker_mode = Some(mode);
            state.picker_search.clear();
        }
        let (title, can_create, create_label) = match mode {
            PickerMode::SetExercise => ("Choose exercise", true, "Create exercise"),
            PickerMode::SetEquipment => ("Choose equipment", true, "Create equipment"),
            PickerMode::AnalyticsExercise => ("Filter by exercise", false, ""),
            PickerMode::AnalyticsMuscle => ("Filter by muscle group", false, ""),
            PickerMode::ExerciseMuscle(role) => (
                match role {
                    MuscleRole::Primary => "Add primary muscle",
                    MuscleRole::Secondary => "Add secondary muscle",
                },
                true,
                "Create muscle group",
            ),
        };
        self.ui.set_picker_title(title.into());
        self.ui.set_picker_can_create(can_create);
        self.ui.set_picker_create_label(create_label.into());
        self.ui.set_picker_search("".into());
        self.ui.set_picker_error("".into());
        self.refresh_picker_rows();
        self.ui.set_picker_visible(true);
    }

    fn refresh_picker_rows(&self) {
        let (mode, search) = {
            let state = self.state.borrow();
            (state.picker_mode, state.picker_search.clone())
        };
        let Some(mode) = mode else {
            return;
        };
        let assigned: HashSet<i64> = self
            .state
            .borrow()
            .exercise_editor_assignments
            .iter()
            .map(|assignment| assignment.muscle_id)
            .collect();

        let conn = self.db.borrow();
        let connection = conn.connection();
        let mut rows: Vec<PickerRow> = Vec::new();

        match mode {
            PickerMode::SetExercise | PickerMode::AnalyticsExercise => {
                let include_inactive = matches!(mode, PickerMode::AnalyticsExercise);
                match library_service::list_exercises(connection, include_inactive, Some(&search)) {
                    Ok(exercises) => {
                        if include_inactive {
                            rows.push(PickerRow {
                                id: 0,
                                title: "Any exercise".into(),
                                subtitle: "Remove the current exercise filter".into(),
                                is_none: true,
                            });
                        }
                        for exercise in exercises {
                            let mut subtitle = exercise.category.as_str().to_string();
                            if !exercise.is_active {
                                subtitle.push_str("  •  archived");
                            }
                            rows.push(PickerRow {
                                id: exercise.id as i32,
                                title: exercise.name.into(),
                                subtitle: subtitle.into(),
                                is_none: false,
                            });
                        }
                    }
                    Err(error) => {
                        drop(conn);
                        self.report_error(&error);
                        return;
                    }
                }
            }
            PickerMode::SetEquipment => {
                match library_service::list_equipment(connection, false, Some(&search)) {
                    Ok(items) => {
                        for item in items {
                            rows.push(PickerRow {
                                id: item.id as i32,
                                title: item.name.into(),
                                subtitle: item
                                    .category
                                    .unwrap_or_else(|| "Uncategorized".into())
                                    .into(),
                                is_none: false,
                            });
                        }
                    }
                    Err(error) => {
                        drop(conn);
                        self.report_error(&error);
                        return;
                    }
                }
            }
            PickerMode::AnalyticsMuscle => {
                match library_service::list_muscle_groups(connection, true, Some(&search)) {
                    Ok(groups) => {
                        rows.push(PickerRow {
                            id: 0,
                            title: "Any muscle group".into(),
                            subtitle: "Remove the current muscle filter".into(),
                            is_none: true,
                        });
                        for group in groups {
                            let mut subtitle =
                                group.description.unwrap_or_else(|| "No description".into());
                            if !group.is_active {
                                subtitle.push_str("  •  archived");
                            }
                            rows.push(PickerRow {
                                id: group.id as i32,
                                title: group.name.into(),
                                subtitle: subtitle.into(),
                                is_none: false,
                            });
                        }
                    }
                    Err(error) => {
                        drop(conn);
                        self.report_error(&error);
                        return;
                    }
                }
            }
            PickerMode::ExerciseMuscle(_) => {
                match library_service::list_muscle_groups(connection, false, Some(&search)) {
                    Ok(groups) => {
                        for group in groups {
                            if assigned.contains(&group.id) {
                                continue;
                            }
                            rows.push(PickerRow {
                                id: group.id as i32,
                                title: group.name.into(),
                                subtitle: group
                                    .description
                                    .unwrap_or_else(|| "No description".into())
                                    .into(),
                                is_none: false,
                            });
                        }
                    }
                    Err(error) => {
                        drop(conn);
                        self.report_error(&error);
                        return;
                    }
                }
            }
        }
        drop(conn);

        self.ui.set_picker_empty(rows.is_empty());
        self.ui.set_picker_rows(ModelRc::new(VecModel::from(rows)));
    }

    fn picker_search_edited(&self, text: SharedString) {
        {
            let mut state = self.state.borrow_mut();
            state.picker_search = text.to_string();
        }
        self.ui.set_picker_error("".into());
        self.refresh_picker_rows();
    }

    fn picker_select(&self, id: i32) {
        let mode = self.state.borrow().picker_mode;
        let Some(mode) = mode else {
            return;
        };
        let id = id as i64;

        match mode {
            PickerMode::SetExercise => {
                let exercise =
                    match library_service::get_exercise(self.db.borrow().connection(), id) {
                        Ok(exercise) => exercise,
                        Err(error) => {
                            self.ui.set_picker_error(error.user_message().into());
                            return;
                        }
                    };
                self.state.borrow_mut().set_exercise_id = id;
                self.ui.set_set_exercise_name(exercise.name.into());
                self.close_picker();
            }
            PickerMode::SetEquipment => {
                let item = match library_service::get_equipment(self.db.borrow().connection(), id) {
                    Ok(item) => item,
                    Err(error) => {
                        self.ui.set_picker_error(error.user_message().into());
                        return;
                    }
                };
                self.state.borrow_mut().set_equipment_id = id;
                self.ui.set_set_equipment_name(item.name.into());
                self.close_picker();
            }
            PickerMode::AnalyticsExercise => {
                self.state.borrow_mut().analytics_exercise = (id > 0).then_some(id);
                self.close_picker();
                self.refresh_analytics();
            }
            PickerMode::AnalyticsMuscle => {
                self.state.borrow_mut().analytics_muscle = (id > 0).then_some(id);
                self.close_picker();
                self.refresh_analytics();
            }
            PickerMode::ExerciseMuscle(role) => {
                let muscle =
                    match library_service::get_muscle_group(self.db.borrow().connection(), id) {
                        Ok(muscle) => muscle,
                        Err(error) => {
                            self.ui.set_picker_error(error.user_message().into());
                            return;
                        }
                    };
                let already_assigned = self
                    .state
                    .borrow()
                    .exercise_editor_assignments
                    .iter()
                    .any(|assignment| assignment.muscle_id == id);
                if already_assigned {
                    self.ui
                        .set_picker_error("That muscle group is already assigned".into());
                    return;
                }
                self.state
                    .borrow_mut()
                    .exercise_editor_assignments
                    .push(AssignmentEdit {
                        muscle_id: id,
                        name: muscle.name,
                        role,
                    });
                self.refresh_assignment_rows();
                self.close_picker();
            }
        }
    }

    fn picker_create(&self, name: SharedString) {
        let name = name.to_string();
        let name = name.trim();
        if name.is_empty() {
            self.ui
                .set_picker_error("Enter a name for the new record".into());
            return;
        }
        let mode = self.state.borrow().picker_mode;
        let Some(mode) = mode else {
            return;
        };
        match mode {
            PickerMode::SetExercise => {
                let result = library_service::quick_create_exercise(
                    self.db.borrow_mut().connection_mut(),
                    name,
                );
                match result
                    .and_then(|id| library_service::get_exercise(self.db.borrow().connection(), id))
                {
                    Ok(exercise) => {
                        self.state.borrow_mut().set_exercise_id = exercise.id;
                        self.ui.set_set_exercise_name(exercise.name.into());
                        self.close_picker();
                        self.toast("Exercise created");
                    }
                    Err(error) => {
                        self.ui.set_picker_error(error.user_message().into());
                    }
                }
            }
            PickerMode::SetEquipment => {
                let result =
                    library_service::quick_create_equipment(self.db.borrow().connection(), name);
                match result.and_then(|id| {
                    library_service::get_equipment(self.db.borrow().connection(), id)
                }) {
                    Ok(item) => {
                        self.state.borrow_mut().set_equipment_id = item.id;
                        self.ui.set_set_equipment_name(item.name.into());
                        self.close_picker();
                        self.toast("Equipment created");
                    }
                    Err(error) => {
                        self.ui.set_picker_error(error.user_message().into());
                    }
                }
            }
            PickerMode::ExerciseMuscle(role) => {
                let result =
                    library_service::quick_create_muscle_group(self.db.borrow().connection(), name);
                match result.and_then(|id| {
                    library_service::get_muscle_group(self.db.borrow().connection(), id)
                }) {
                    Ok(muscle) => {
                        self.state
                            .borrow_mut()
                            .exercise_editor_assignments
                            .push(AssignmentEdit {
                                muscle_id: muscle.id,
                                name: muscle.name,
                                role,
                            });
                        self.refresh_assignment_rows();
                        self.close_picker();
                        self.toast("Muscle group created");
                    }
                    Err(error) => {
                        self.ui.set_picker_error(error.user_message().into());
                    }
                }
            }
            PickerMode::AnalyticsExercise | PickerMode::AnalyticsMuscle => {}
        }
    }

    fn close_picker(&self) {
        self.state.borrow_mut().picker_mode = None;
        self.ui.set_picker_visible(false);
        self.ui.set_picker_error("".into());
    }

    fn picker_cancel(&self) {
        self.close_picker();
    }

    // ---------------------------------------------------------------------
    // Profile & measurements
    // ---------------------------------------------------------------------

    fn refresh_profile(&self) {
        let user = match profile_service::get_local_user(self.db.borrow().connection()) {
            Ok(user) => user,
            Err(error) => {
                self.report_error(&error);
                return;
            }
        };
        let latest = match profile_service::latest_metrics(self.db.borrow().connection()) {
            Ok(latest) => latest,
            Err(error) => {
                self.report_error(&error);
                return;
            }
        };
        let comparisons = match profile_service::comparisons(self.db.borrow().connection()) {
            Ok(comparisons) => comparisons,
            Err(error) => {
                self.report_error(&error);
                return;
            }
        };
        let history = match profile_service::measurement_history(self.db.borrow().connection()) {
            Ok(history) => history,
            Err(error) => {
                self.report_error(&error);
                return;
            }
        };

        let height = latest.height_cm.or(user.height_cm);
        self.ui.set_profile_name(user.name.into());
        self.ui.set_profile_height(
            height
                .map(|value| format!("{} cm", format::number(value, 1)))
                .unwrap_or_else(|| "-".into())
                .into(),
        );
        self.ui
            .set_profile_dob(user.date_of_birth.unwrap_or_else(|| "-".into()).into());
        self.ui.set_profile_latest_date(
            latest
                .latest_measurement_date
                .map(|date| format!("Latest snapshot: {}", display_date(date)))
                .unwrap_or_else(|| "No measurements yet".into())
                .into(),
        );
        self.ui
            .set_profile_weight(format::optional_kg(latest.weight_kg).into());
        self.ui.set_profile_bmi(
            latest
                .bmi
                .map(|value| format::number(value, 1))
                .unwrap_or_else(|| "-".into())
                .into(),
        );
        self.ui.set_profile_bmi_category(if latest.bmi.is_some() {
            bmi_category(latest.bmi).into()
        } else {
            "".into()
        });
        self.ui.set_profile_body_fat(
            latest
                .body_fat_percentage
                .map(|value| format!("{} %", format::number(value, 1)))
                .unwrap_or_else(|| "-".into())
                .into(),
        );
        for (property, value) in [
            (ProfileMetric::Waist, latest.waist_cm),
            (ProfileMetric::Chest, latest.chest_cm),
            (ProfileMetric::Arm, latest.arm_cm),
            (ProfileMetric::Thigh, latest.thigh_cm),
        ] {
            let text = value
                .map(|value| format!("{} cm", format::number(value, 1)))
                .unwrap_or_else(|| "-".into());
            match property {
                ProfileMetric::Waist => self.ui.set_profile_waist(text.into()),
                ProfileMetric::Chest => self.ui.set_profile_chest(text.into()),
                ProfileMetric::Arm => self.ui.set_profile_arm(text.into()),
                ProfileMetric::Thigh => self.ui.set_profile_thigh(text.into()),
            }
        }

        let comparison_rows: Vec<ComparisonRow> = comparisons.iter().map(comparison_row).collect();
        self.ui
            .set_profile_comparisons(ModelRc::new(VecModel::from(comparison_rows)));

        let measurement_rows: Vec<MeasurementRow> = history
            .iter()
            .map(|measurement| {
                let detail = measurement_detail(measurement);
                let notes = measurement.notes.clone().unwrap_or_default();
                MeasurementRow {
                    title: display_date(measurement.measured_at).into(),
                    detail: detail.into(),
                    has_notes: !notes.trim().is_empty(),
                    notes: notes.into(),
                }
            })
            .collect();
        self.ui
            .set_profile_measurements_empty(measurement_rows.is_empty());
        self.ui
            .set_profile_measurements(ModelRc::new(VecModel::from(measurement_rows)));
    }

    fn open_profile_edit(&self) {
        let user = match profile_service::get_local_user(self.db.borrow().connection()) {
            Ok(user) => user,
            Err(error) => {
                self.report_error(&error);
                return;
            }
        };
        self.ui.set_profile_edit_name(user.name.into());
        self.ui.set_profile_edit_height(
            user.height_cm
                .map(|value| format::number(value, 1))
                .unwrap_or_default()
                .into(),
        );
        self.ui
            .set_profile_edit_dob(user.date_of_birth.unwrap_or_default().into());
        self.ui.set_profile_edit_error("".into());
        self.ui.set_profile_edit_visible(true);
    }

    fn save_profile(&self) {
        let name = self.ui.get_profile_edit_name().to_string();
        let height = match parse_optional_f64(&self.ui.get_profile_edit_height(), "Height") {
            Ok(height) => height,
            Err(error) => {
                self.ui.set_profile_edit_error(error.user_message().into());
                return;
            }
        };
        let date_of_birth =
            match parse_optional_date_field(&self.ui.get_profile_edit_dob(), "Date of birth") {
                Ok(date) => date,
                Err(error) => {
                    self.ui.set_profile_edit_error(error.user_message().into());
                    return;
                }
            };
        match profile_service::update_profile(
            self.db.borrow().connection(),
            &name,
            height,
            date_of_birth,
        ) {
            Ok(()) => {
                self.ui.set_profile_edit_visible(false);
                self.ui.set_profile_edit_error("".into());
                self.refresh_profile();
                self.toast("Profile updated");
            }
            Err(error) => {
                self.ui.set_profile_edit_error(error.user_message().into());
            }
        }
    }

    fn cancel_profile_edit(&self) {
        self.ui.set_profile_edit_visible(false);
        self.ui.set_profile_edit_error("".into());
    }

    fn open_measurement_form(&self) {
        self.ui
            .set_measurement_date(format_date(today_local()).into());
        self.ui.set_measurement_weight("".into());
        self.ui.set_measurement_height("".into());
        self.ui.set_measurement_body_fat("".into());
        self.ui.set_measurement_waist("".into());
        self.ui.set_measurement_chest("".into());
        self.ui.set_measurement_hip("".into());
        self.ui.set_measurement_arm("".into());
        self.ui.set_measurement_thigh("".into());
        self.ui.set_measurement_notes("".into());
        self.ui.set_measurement_error("".into());
        self.ui.set_measurement_form_visible(true);
    }

    fn save_measurement(&self) {
        let date = match parse_date_field(&self.ui.get_measurement_date(), "Measurement date") {
            Ok(date) => date,
            Err(error) => {
                self.ui.set_measurement_error(error.user_message().into());
                return;
            }
        };
        let fields = [
            ("Weight", self.ui.get_measurement_weight()),
            ("Height", self.ui.get_measurement_height()),
            ("Body fat", self.ui.get_measurement_body_fat()),
            ("Waist", self.ui.get_measurement_waist()),
            ("Chest", self.ui.get_measurement_chest()),
            ("Hip", self.ui.get_measurement_hip()),
            ("Arm", self.ui.get_measurement_arm()),
            ("Thigh", self.ui.get_measurement_thigh()),
        ];
        let mut values: Vec<Option<f64>> = Vec::with_capacity(fields.len());
        for (label, value) in &fields {
            match parse_optional_f64(value, label) {
                Ok(value) => values.push(value),
                Err(error) => {
                    self.ui.set_measurement_error(error.user_message().into());
                    return;
                }
            }
        }
        let notes = self.ui.get_measurement_notes().to_string();
        let input = NewBodyMeasurement {
            measured_at: date,
            weight_kg: values[0],
            height_cm: values[1],
            body_fat_percentage: values[2],
            waist_cm: values[3],
            chest_cm: values[4],
            hip_cm: values[5],
            arm_cm: values[6],
            thigh_cm: values[7],
            notes,
        };
        match profile_service::add_measurement(self.db.borrow().connection(), &input) {
            Ok(_) => {
                self.ui.set_measurement_form_visible(false);
                self.ui.set_measurement_error("".into());
                self.refresh_profile();
                self.toast("Measurement snapshot appended");
            }
            Err(error) => {
                self.ui.set_measurement_error(error.user_message().into());
            }
        }
    }

    fn cancel_measurement_form(&self) {
        self.ui.set_measurement_form_visible(false);
        self.ui.set_measurement_error("".into());
    }

    // ---------------------------------------------------------------------
    // Updates
    // ---------------------------------------------------------------------

    fn refresh_update_view(&self) {
        let installed = Version::parse(env!("CARGO_PKG_VERSION"))
            .map(|version| format!("v{version}"))
            .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").into());
        self.ui.set_update_installed(installed.clone().into());

        let config_path = self.data_dir.join("updates").join("config.json");
        self.ui
            .set_update_config_path(config_path.display().to_string().into());

        let service = match UpdateService::new(&self.data_dir, self.db_path(), self.schema_version)
        {
            Ok(service) => service,
            Err(error) => {
                self.ui.set_update_status(error.user_message().into());
                self.ui.set_update_can_download(false);
                self.ui.set_update_can_install(false);
                self.ui.set_update_can_download_rollback(false);
                self.ui.set_update_can_install_rollback(false);
                self.ui.set_update_latest("-".into());
                self.ui.set_update_notes("".into());
                self.ui.set_update_staged("".into());
                self.ui.set_update_backup("".into());
                self.ui.set_update_last_checked("never".into());
                self.ui.set_update_running(false);
                return;
            }
        };

        let state = service.state();
        let latest_version = state.latest_version.clone().unwrap_or_else(|| "-".into());
        self.ui.set_update_latest(latest_version.into());
        self.ui.set_update_notes(state.latest_notes.clone().into());
        self.ui.set_update_last_checked(
            state
                .last_checked_at
                .clone()
                .unwrap_or_else(|| "never".into())
                .into(),
        );

        let staged = state.staged.as_ref();
        let rollback_staged = state.rollback_staged.as_ref();
        self.ui.set_update_staged(
            staged
                .map(|package| {
                    format!(
                        "Staged update: v{}  •  verified  •  {}",
                        package.version, package.path
                    )
                })
                .unwrap_or_default()
                .into(),
        );
        self.ui.set_update_backup(
            state
                .pre_install_backup
                .as_ref()
                .map(|path| format!("Last database backup: {path}"))
                .unwrap_or_default()
                .into(),
        );

        let manifest = service.latest_manifest().ok();
        let manifest_has_rollback = manifest
            .as_ref()
            .and_then(|manifest| manifest.rollback.as_ref())
            .and_then(|rollback| rollback.version().ok())
            .map(|version| {
                Version::parse(env!("CARGO_PKG_VERSION"))
                    .map(|installed| version < installed)
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        let rollback_available =
            self.state.borrow().update_rollback_available || manifest_has_rollback;

        let latest_is_newer = state
            .latest_version
            .as_deref()
            .and_then(|version| Version::parse(version.trim_start_matches('v')).ok())
            .zip(Version::parse(env!("CARGO_PKG_VERSION")).ok())
            .map(|(latest, installed)| latest > installed)
            .unwrap_or(false);

        let running = self.state.borrow().update_running;
        let message = self.state.borrow().update_message.clone();
        self.ui.set_update_running(running);
        self.ui.set_update_can_download(!running && latest_is_newer);
        self.ui.set_update_can_install(!running && staged.is_some());
        self.ui.set_update_can_download_rollback(
            !running && rollback_staged.is_none() && rollback_available,
        );
        self.ui
            .set_update_can_install_rollback(!running && rollback_staged.is_some());

        let status = if !message.is_empty() {
            message
        } else if running {
            "Update task running...".to_string()
        } else if let Some(package) = staged {
            format!(
                "Update v{} is downloaded, verified, and staged. Installation will back up the database first.",
                package.version
            )
        } else if state.last_checked_at.is_some() {
            match state.latest_version.as_deref() {
                Some(version) => format!(
                    "Latest checked release: v{}. {}",
                    version.trim_start_matches('v'),
                    if latest_is_newer {
                        "A newer version is available."
                    } else {
                        "Installed version is current."
                    }
                ),
                None => "No release information is available.".into(),
            }
        } else {
            "Not checked yet. Configure a release source in the update config file, then choose Check.".to_string()
        };
        self.ui.set_update_status(status.into());
    }

    fn begin_update(&self, task: UpdateTask) {
        {
            let mut state = self.state.borrow_mut();
            if state.update_running {
                return;
            }
            state.update_running = true;
            state.update_message = task.action_label().into();
        }
        let (sender, receiver) = mpsc::channel();
        self.state.borrow_mut().update_rx = Some(receiver);

        self.ui.set_update_running(true);
        self.ui.set_update_progress(0.0);
        self.ui.set_update_progress_label("".into());
        self.refresh_update_view();

        let data_dir = self.data_dir.clone();
        let db_path = self.db_path();
        let schema = self.schema_version;
        thread::spawn(move || {
            run_update_task(data_dir, db_path, schema, task, sender);
        });
    }

    fn check_updates(&self) {
        self.begin_update(UpdateTask::Check);
    }

    fn download_update(&self) {
        self.begin_update(UpdateTask::Download);
    }

    fn install_update(&self) {
        self.begin_update(UpdateTask::Install);
    }

    fn download_rollback(&self) {
        self.begin_update(UpdateTask::DownloadRollback);
    }

    fn install_rollback(&self) {
        self.begin_update(UpdateTask::InstallRollback);
    }

    fn poll_updates(&self) {
        let mut events = Vec::new();
        let mut disconnected = false;
        {
            let state = self.state.borrow();
            let Some(receiver) = state.update_rx.as_ref() else {
                return;
            };
            loop {
                match receiver.try_recv() {
                    Ok(event) => events.push(event),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }

        let mut finished = false;
        for event in events {
            match event {
                UpdateEvent::Progress { downloaded, total } => {
                    let progress = total
                        .filter(|total| *total > 0)
                        .map(|total| downloaded as f64 / total as f64)
                        .unwrap_or(0.0);
                    let label = match total {
                        Some(total) if total > 0 => format!(
                            "{} of {} ({:.0}%)",
                            format::bytes(downloaded),
                            format::bytes(total),
                            progress * 100.0
                        ),
                        _ => format!("{} downloaded", format::bytes(downloaded)),
                    };
                    self.ui.set_update_progress(progress.clamp(0.0, 1.0) as f32);
                    self.ui.set_update_progress_label(label.into());
                }
                UpdateEvent::Finished {
                    message,
                    rollback_available,
                } => {
                    finished = true;
                    {
                        let mut state = self.state.borrow_mut();
                        state.update_message = message.clone();
                        if let Some(available) = rollback_available {
                            state.update_rollback_available = available;
                        }
                    }
                    self.toast(message);
                }
                UpdateEvent::Failed(message) => {
                    finished = true;
                    let message = format!("Update task failed: {message}");
                    self.state.borrow_mut().update_message = message.clone();
                    self.toast_error(message);
                }
            }
        }

        if finished || disconnected {
            {
                let mut state = self.state.borrow_mut();
                state.update_running = false;
                state.update_rx = None;
            }
            self.ui.set_update_running(false);
            self.ui.set_update_progress(0.0);
            self.ui.set_update_progress_label("".into());
            if disconnected && !finished {
                let message = "Update task stopped unexpectedly".to_string();
                self.state.borrow_mut().update_message = message.clone();
                self.toast_error(message);
            }
            self.refresh_update_view();
        }
    }
}

#[derive(Clone, Copy)]
enum ProfileMetric {
    Waist,
    Chest,
    Arm,
    Thigh,
}

fn range_label(range: DateRange) -> String {
    match (range.start, range.end) {
        (Some(start), Some(end)) => format!("{} – {}", display_date(start), display_date(end)),
        _ => "All recorded history".into(),
    }
}

fn chart_key(chart: AnalyticsChart) -> &'static str {
    match chart {
        AnalyticsChart::WeightProgression => "weight",
        AnalyticsChart::VolumeProgression => "volume",
        AnalyticsChart::WeeklyVolume => "weekly-volume",
        AnalyticsChart::MuscleDistribution => "muscle",
        AnalyticsChart::ExerciseFrequency => "frequency",
    }
}

fn muscle_stat_row(stats: &MuscleGroupStats) -> MuscleStatRow {
    MuscleStatRow {
        name: stats.name.clone().into(),
        sets: format!(
            "Primary sets: {}  •  Secondary sets: {}  •  Trained on {} {}",
            stats.primary_sets,
            stats.secondary_sets,
            stats.workout_days,
            if stats.workout_days == 1 {
                "day"
            } else {
                "days"
            }
        )
        .into(),
        primary: format!(
            "Primary volume (100%): {}",
            format::kg(stats.primary_volume)
        )
        .into(),
        secondary: format!(
            "Secondary volume (50%): {}",
            format::kg(stats.secondary_volume)
        )
        .into(),
        total: format!(
            "Attributed total: {}",
            format::kg(stats.total_volume_attributed())
        )
        .into(),
    }
}

fn metric_unit(metric: MeasurementMetric) -> &'static str {
    match metric {
        MeasurementMetric::Weight => "kg",
        MeasurementMetric::Bmi => "",
        MeasurementMetric::BodyFat => "%",
        _ => "cm",
    }
}

fn comparison_row(comparison: &MetricComparison) -> ComparisonRow {
    let unit = metric_unit(comparison.metric);
    let current = match comparison.current {
        Some(value) => {
            if unit.is_empty() {
                format::number(value, 1)
            } else {
                format!("{} {}", format::number(value, 1), unit)
            }
        }
        None => "-".into(),
    };
    let delta = comparison
        .delta()
        .map(|value| format::signed(value, 1, unit))
        .unwrap_or_else(|| "No comparison".into());
    let dates = match (comparison.previous_date, comparison.current_date) {
        (Some(previous), Some(current)) => {
            format!("{} to {}", display_date(previous), display_date(current))
        }
        (None, Some(current)) => format!("First record {}", display_date(current)),
        _ => String::new(),
    };
    ComparisonRow {
        label: comparison.metric.label().into(),
        current: current.into(),
        delta: delta.into(),
        has_delta: comparison.delta().is_some(),
        dates: dates.into(),
    }
}

fn measurement_detail(measurement: &crate::models::body_measurement::BodyMeasurement) -> String {
    let mut parts = Vec::new();
    if let Some(value) = measurement.weight_kg {
        parts.push(format::kg(value));
    }
    if let Some(value) = measurement.height_cm {
        parts.push(format!("height {} cm", format::number(value, 1)));
    }
    if let Some(value) = measurement.body_fat_percentage {
        parts.push(format!("{} % body fat", format::number(value, 1)));
    }
    if let Some(value) = measurement.bmi() {
        parts.push(format!("BMI {}", format::number(value, 1)));
    }
    for (label, value) in [
        ("waist", measurement.waist_cm),
        ("chest", measurement.chest_cm),
        ("hip", measurement.hip_cm),
        ("arm", measurement.arm_cm),
        ("thigh", measurement.thigh_cm),
    ] {
        if let Some(value) = value {
            parts.push(format!("{label} {} cm", format::number(value, 1)));
        }
    }
    if parts.is_empty() {
        "Notes only".into()
    } else {
        parts.join("  •  ")
    }
}

fn parse_optional_f64(value: &SharedString, label: &str) -> AppResult<Option<f64>> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    value
        .parse::<f64>()
        .map(Some)
        .map_err(|_| AppError::validation(format!("{label} must be a number")))
}

fn parse_optional_i64(value: &SharedString, label: &str) -> AppResult<Option<i64>> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    value
        .parse::<i64>()
        .map(Some)
        .map_err(|_| AppError::validation(format!("{label} must be a whole number")))
}

// -------------------------------------------------------------------------
// Update worker
// -------------------------------------------------------------------------

fn run_update_task(
    data_dir: PathBuf,
    db_path: PathBuf,
    schema: i64,
    task: UpdateTask,
    sender: Sender<UpdateEvent>,
) {
    let mut service = match UpdateService::new(&data_dir, &db_path, schema) {
        Ok(service) => service,
        Err(error) => {
            let _ = sender.send(UpdateEvent::Failed(error.user_message()));
            return;
        }
    };

    match task {
        UpdateTask::Check => match service.check() {
            Ok(check) => {
                let message = check_message(&check);
                let _ = sender.send(UpdateEvent::Finished {
                    message,
                    rollback_available: Some(check.rollback_available),
                });
            }
            Err(error) => {
                let _ = sender.send(UpdateEvent::Failed(error.user_message()));
            }
        },
        UpdateTask::Download => {
            let result = {
                let mut progress = |downloaded: u64, total: Option<u64>| {
                    let _ = sender.send(UpdateEvent::Progress { downloaded, total });
                };
                service.download_latest(&mut progress)
            };
            match result {
                Ok(outcome) => {
                    let _ = sender.send(UpdateEvent::Finished {
                        message: format!(
                            "Downloaded and verified update v{}. Package staged at {}.",
                            outcome.staged.version,
                            outcome.path.display()
                        ),
                        rollback_available: None,
                    });
                }
                Err(error) => {
                    let _ = sender.send(UpdateEvent::Failed(error.user_message()));
                }
            }
        }
        UpdateTask::Install => match service.install_staged(false) {
            Ok(outcome) => {
                let _ = sender.send(UpdateEvent::Finished {
                    message: outcome.message,
                    rollback_available: None,
                });
            }
            Err(error) => {
                let _ = sender.send(UpdateEvent::Failed(error.user_message()));
            }
        },
        UpdateTask::DownloadRollback => {
            let result = {
                let mut progress = |downloaded: u64, total: Option<u64>| {
                    let _ = sender.send(UpdateEvent::Progress { downloaded, total });
                };
                service.download_rollback(&mut progress)
            };
            match result {
                Ok(outcome) => {
                    let _ = sender.send(UpdateEvent::Finished {
                        message: format!(
                            "Downloaded and verified rollback package v{}. Staged at {}.",
                            outcome.staged.version,
                            outcome.path.display()
                        ),
                        rollback_available: None,
                    });
                }
                Err(error) => {
                    let _ = sender.send(UpdateEvent::Failed(error.user_message()));
                }
            }
        }
        UpdateTask::InstallRollback => match service.install_staged(true) {
            Ok(outcome) => {
                let _ = sender.send(UpdateEvent::Finished {
                    message: outcome.message,
                    rollback_available: None,
                });
            }
            Err(error) => {
                let _ = sender.send(UpdateEvent::Failed(error.user_message()));
            }
        },
    }
}

fn check_message(check: &UpdateCheck) -> String {
    let mut message = match check.availability {
        UpdateAvailability::Available => match &check.latest_version {
            Some(version) => format!(
                "Update available: v{} (installed v{}).",
                version, check.current_version
            ),
            None => "Update available.".into(),
        },
        UpdateAvailability::UpToDate => format!(
            "Up to date: installed v{} is the latest release.",
            check.current_version
        ),
        UpdateAvailability::NotConfigured => {
            "Update source not configured. Add a release source to the update config file.".into()
        }
        UpdateAvailability::IncompatibleDatabase => match &check.latest_version {
            Some(version) => format!(
                "Release v{} requires a different database schema. This installation uses a schema outside the supported range; the update was not downloaded.",
                version
            ),
            None => "The release is incompatible with this database schema.".into(),
        },
        UpdateAvailability::UnsupportedPlatform => {
            "The release has no package for this platform".into()
        }
        UpdateAvailability::AlreadyNewer => {
            "Installed build is newer than the release channel".into()
        }
    };
    if !check.notes.trim().is_empty() {
        message.push(' ');
        message.push_str(check.notes.trim());
    }
    message
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_parsers_accept_empty_values() {
        let empty: SharedString = "".into();
        assert_eq!(parse_optional_f64(&empty, "Weight").unwrap(), None);
        assert_eq!(parse_optional_i64(&empty, "Reps").unwrap(), None);
    }

    #[test]
    fn optional_parsers_reject_garbage() {
        let bad: SharedString = "abc".into();
        assert!(parse_optional_f64(&bad, "Weight").is_err());
        assert!(parse_optional_i64(&bad, "Reps").is_err());
    }

    #[test]
    fn range_label_handles_unbounded_ranges() {
        assert_eq!(range_label(DateRange::all_time()), "All recorded history");
    }
}
