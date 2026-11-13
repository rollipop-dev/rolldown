// An instance the index re-exports and also reads in its own helper.
(globalThis.__rollipop_side_effect_barrel_log ??= []).push('i18n');
export default { t: (key) => key.toUpperCase() };
