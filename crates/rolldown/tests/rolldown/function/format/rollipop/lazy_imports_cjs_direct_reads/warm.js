// A side-effect import stays eager, so hot.cjs has loaded before main reads it.
import './hot.cjs';
