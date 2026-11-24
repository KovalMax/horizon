import { useState } from "react";
import { Icon } from "../../../shared/ui";

const presets = [
  [15, "15 minutes before"],
  [30, "30 minutes before"],
  [60, "1 hour before"],
  [120, "2 hours before"],
  [1440, "1 day before"],
  [2880, "2 days before"],
  [10080, "1 week before"],
] as const;

export function reminderOffsetsFromForm(form: FormData, name: string) {
  return form.getAll(name).filter(Boolean).map(Number);
}

export function ReminderPicker({
  name,
  legend = "Reminders",
  initialOffsets = [],
  disabled = false,
}: {
  name: string;
  legend?: string;
  initialOffsets?: number[];
  disabled?: boolean;
}) {
  const [offsets, setOffsets] = useState(initialOffsets);
  const rows = offsets.length < 5 ? [...offsets, null] : offsets;

  return (
    <fieldset className="reminder-picker" hidden={disabled} disabled={disabled}>
      <legend>{legend}</legend>
      <div className="reminder-rows">
        {rows.map((offset, index) => (
          <label className="reminder-row" key={index}>
            <Icon name="bell" size={18} />
            <span>Reminder</span>
            <select
              aria-label={`Reminder ${index + 1}`}
              name={name}
              value={offset ?? ""}
              onChange={(event) => {
                const value = event.currentTarget.value;
                setOffsets((current) =>
                  value
                    ? [
                        ...current.slice(0, index),
                        Number(value),
                        ...current.slice(index + 1),
                      ]
                    : current.filter((_, i) => i !== index),
                );
              }}
            >
              <option value="">None</option>
              {presets.map(([value, label]) => (
                <option
                  key={value}
                  value={value}
                  disabled={value !== offset && offsets.includes(value)}
                >
                  {label}
                </option>
              ))}
            </select>
          </label>
        ))}
      </div>
    </fieldset>
  );
}
