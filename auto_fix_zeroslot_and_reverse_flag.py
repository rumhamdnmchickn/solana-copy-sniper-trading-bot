import pathlib
from datetime import datetime

path = pathlib.Path("src/processor/sniper_bot.rs")
backup = path.with_suffix(".rs.bak_" + datetime.utcnow().strftime("%Y%m%d%H%M%S"))

text = path.read_text(encoding="utf-8")

backup.write_text(text, encoding="utf-8")

# 1) Old style zeroslot call → pass &app_state instead
text = text.replace(
    "app_state.zeroslot_rpc_client.clone(),",
    "&app_state,",
)

# 2) Field name: is_reverse_when_pump_swap → _is_reverse_when_pump_swap
text = text.replace(
    "is_reverse_when_pump_swap",
    "_is_reverse_when_pump_swap",
)

path.write_text(text, encoding="utf-8")

print(f"Backup written to {backup}")
print("sniper_bot.rs updated.")
