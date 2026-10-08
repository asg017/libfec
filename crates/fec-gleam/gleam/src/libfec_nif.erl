%% Loads the Rust NIF (crates/fec-gleam) from this package's priv/ dir.
%% Every NIF in src/lib.rs needs a stub export here.
-module(libfec_nif).
-export([version/0, open/2, next_batch/2, skipped/1, close/1]).
-on_load(init/0).

init() ->
    Priv = case code:priv_dir(libfec) of
        {error, bad_name} -> "priv";
        Dir -> Dir
    end,
    erlang:load_nif(filename:join(Priv, "libfec_nif"), 0).

version() -> erlang:nif_error(nif_not_loaded).
open(_Path, _SkipUnknown) -> erlang:nif_error(nif_not_loaded).
next_batch(_Handle, _N) -> erlang:nif_error(nif_not_loaded).
skipped(_Handle) -> erlang:nif_error(nif_not_loaded).
close(_Handle) -> erlang:nif_error(nif_not_loaded).
