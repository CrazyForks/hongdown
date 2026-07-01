import { Component, For } from "solid-js";

interface ToggleProps {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}

export const Toggle: Component<ToggleProps> = (props) => {
  return (
    <label class="flex items-center gap-2 cursor-pointer select-none">
      <div class="relative inline-flex items-center cursor-pointer">
        <input
          type="checkbox"
          class="sr-only peer"
          checked={props.checked}
          onChange={(e) => props.onChange(e.currentTarget.checked)}
        />
        <div class="w-9 h-5 bg-paper-shade peer-focus:outline-none peer-focus:ring-2 peer-focus:ring-hong/60 rounded-full peer dark:bg-night-shade peer-checked:after:translate-x-full after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-paper after:rounded-full after:h-4 after:w-4 after:transition-all after:shadow-sm peer-checked:bg-hong dark:peer-checked:bg-hong"></div>
      </div>
      <span class="text-sm font-mono">{props.label}</span>
    </label>
  );
};

const LABEL_CLASS =
  "font-mono text-xs tracking-wide text-quiet";

interface SelectProps {
  label: string;
  value: string;
  options: { label: string; value: string }[];
  onChange: (value: string) => void;
}

export const Select: Component<SelectProps> = (props) => {
  return (
    <div class="flex flex-col gap-1">
      <label class={LABEL_CLASS}>{props.label}</label>
      <select
        class="input-base text-sm font-mono"
        value={props.value}
        onChange={(e) => props.onChange(e.currentTarget.value)}
      >
        <For each={props.options}>
          {(opt) => <option value={opt.value}>{opt.label}</option>}
        </For>
      </select>
    </div>
  );
};

interface NumberInputProps {
  label: string;
  value: number;
  min?: number;
  max?: number;
  onChange: (value: number) => void;
}

export const NumberInput: Component<NumberInputProps> = (props) => {
  return (
    <div class="flex flex-col gap-1">
      <label class={LABEL_CLASS}>{props.label}</label>
      <input
        type="number"
        class="input-base text-sm font-mono"
        value={props.value}
        min={props.min}
        max={props.max}
        onInput={(e) => props.onChange(Number(e.currentTarget.value))}
      />
    </div>
  );
};

interface TextInputProps {
  label: string;
  value: string;
  placeholder?: string;
  onChange: (value: string) => void;
}

export const TextInput: Component<TextInputProps> = (props) => {
  return (
    <div class="flex flex-col gap-1">
      <label class={LABEL_CLASS}>{props.label}</label>
      <input
        type="text"
        class="input-base text-sm font-mono"
        value={props.value}
        placeholder={props.placeholder}
        onInput={(e) => props.onChange(e.currentTarget.value)}
      />
    </div>
  );
};

interface TextAreaInputProps {
  label: string;
  value: string[];
  placeholder?: string;
  rows?: number;
  onChange: (value: string[]) => void;
}

export const TextAreaInput: Component<TextAreaInputProps> = (props) => {
  const textValue = () => props.value.join("\n");

  const handleChange = (text: string) => {
    const lines = text
      .split("\n")
      .map((line) => line.trim())
      .filter((line) => line.length > 0);
    props.onChange(lines);
  };

  return (
    <div class="flex flex-col gap-1">
      <label class={LABEL_CLASS}>{props.label}</label>
      <textarea
        class="input-base text-sm resize-none font-mono"
        rows={props.rows ?? 3}
        placeholder={props.placeholder}
        value={textValue()}
        onInput={(e) => handleChange(e.currentTarget.value)}
      />
    </div>
  );
};
