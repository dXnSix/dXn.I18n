---
title: Dates and times
description: Format dates and times with :date, :time and :datetime - date fields and styles, time precision, 12 and 24 hour clocks, ISO 8601 text operands, calendars and why values are wall times without a time zone.
group: Functions
order: 80
---

`:date`, `:time` and `:datetime` format a wall time in the message's locale: its month and weekday names, field order, clock and calendar.

```rust
use dxn_i18n::{Args, Message};
use dxn_intl::{DateTime, parse_locale};

let when = DateTime::new(2026, 10, 27, 15, 45, 30)?;
let args = Args::new().with("when", when);
let message = Message::parse("{$when :date style=long}, {$when :time}")?;

assert_eq!(message.format(&parse_locale("en-US")?, &args), "October 27, 2026, 3:45\u{202f}PM");
assert_eq!(message.format(&parse_locale("de-DE")?, &args), "27. Oktober 2026, 15:45");
assert_eq!(message.format(&parse_locale("ja")?, &args), "2026年10月27日, 15:45");
```

The `U+202F` before `PM` is the narrow no-break space that current CLDR data puts there.

## Operands: DateTime or ISO 8601 text

A date is a `dxn_intl::DateTime`: year, month, day, hour, minute, second and nanosecond on the ISO calendar. Build it with `DateTime::date(y, m, d)` (midnight) or `DateTime::new(y, m, d, h, min, s)`; both check that the date exists. A message can also take text:

| Text | Reads as |
|---|---|
| `2026-10-27` | Midnight at the start of that day. |
| `2026-10-27T15:45` | A date and time; seconds are optional. |
| `2026-10-27T15:45:30.5` | With a fraction of up to nine digits. |

The separator is `T` (or `t`). Text with a zone (`...Z`, `...+01:00`), a space instead of `T`, or a day that does not exist is a bad operand. Literals work too: `{|2026-10-27T09:05| :time}`.

### There is no time zone

A `DateTime` is the clock on the wall: no zone, no offset. Convert an instant to the user's local time first, in your own code, with the offset the user's zone has at that instant:

```rust
use dxn_i18n::{Args, Message};
use dxn_intl::{DateTime, parse_locale};

// 2026-10-27T15:45:00Z, shown in UTC+1.
let local = DateTime::from_unix_millis(1_793_115_900_000, 3600)?;
let message = Message::parse("{$when :time}")?;
let args = Args::new().with("when", local);
assert_eq!(message.format(&parse_locale("de")?, &args), "16:45");
```

The library has no zone database and no zone names. A time zone crate can supply the offset.

## :date

| Option | Values | Default |
|---|---|---|
| `style` | `full`, `long`, `medium`, `short`. `full` adds the weekday. | `medium` |
| `length` | The same values; `style` wins when both are given. | |
| `fields` | `year-month-day`, `year-month-day-weekday`, `month-day`, `month-day-weekday`, `day`, `day-weekday`, `weekday`, `month`, `year-month`, `year` | `year-month-day` |

`style` and `length` set how long the words are; `fields` sets which parts show. They combine:

```rust
use dxn_i18n::{Args, Message};
use dxn_intl::{DateTime, parse_locale};

let args = Args::new().with("d", DateTime::new(2026, 10, 27, 15, 45, 30)?);
let en = parse_locale("en-US")?;
let show = |source: &str| Message::parse(source).unwrap().format(&en, &args);

assert_eq!(show("{$d :date}"), "Oct 27, 2026");
assert_eq!(show("{$d :date style=full}"), "Tuesday, October 27, 2026");
assert_eq!(show("{$d :date style=short}"), "10/27/26");
assert_eq!(show("{$d :date fields=month-day length=long}"), "October 27");
assert_eq!(show("{$d :date fields=year-month length=long}"), "October 2026");
assert_eq!(show("{$d :date fields=weekday}"), "Tue");
```

## :time

| Option | Values | Default |
|---|---|---|
| `style` | `short` (to the minute), `medium`, `long`, `full` (to the second) | `short` |
| `precision` | `hour`, `minute`, `second`; wins over `style` | `minute` |
| `hour12` | `true` for a 12-hour clock, `false` for 24-hour | the locale's clock |
| `hourCycle` | `h11`, `h12`, `h23`, `h24`; wins over `hour12` | the locale's clock |

```rust
use dxn_i18n::{Args, Message};
use dxn_intl::{DateTime, parse_locale};

let args = Args::new().with("t", DateTime::new(2026, 10, 27, 15, 45, 30)?);
let show = |tag: &str, source: &str| {
    Message::parse(source).unwrap().format(&parse_locale(tag).unwrap(), &args)
};

assert_eq!(show("en-US", "{$t :time}"), "3:45\u{202f}PM");
assert_eq!(show("en-US", "{$t :time precision=hour}"), "3\u{202f}PM");
assert_eq!(show("en-US", "{$t :time precision=second hour12=false}"), "15:45:30");
assert_eq!(show("en-GB", "{$t :time}"), "15:45");
assert_eq!(show("en-US-u-hc-h23", "{$t :time}"), "15:45");
```

The last line shows the user's own clock preference: a `-u-hc-` extension on the locale applies to every time the message shows, with no option in the message. A `hour12` or `hourCycle` option in the message overrides it.

## :datetime

`:datetime` shows a date, a time or both, with the date options prefixed `date` and the time options prefixed `time`.

| Option | Same as | Values |
|---|---|---|
| `dateStyle` | `:date style` | `full`, `long`, `medium`, `short` |
| `dateLength` | `:date length` | as `dateStyle` |
| `dateFields` | `:date fields` | as `:date fields` |
| `timeStyle` | `:time style` | `full`, `long`, `medium`, `short` |
| `timePrecision` | `:time precision` | `hour`, `minute`, `second` |
| `hour12`, `hourCycle` | the same on `:time` | |

With no date or time option, `:datetime` shows a medium date and a time to the minute. With only date options it shows only the date, with only time options only the time, and with both, both:

```rust
use dxn_i18n::{Args, Message};
use dxn_intl::{DateTime, parse_locale};

let args = Args::new().with("d", DateTime::new(2026, 10, 27, 15, 45, 30)?);
let show = |tag: &str, source: &str| {
    Message::parse(source).unwrap().format(&parse_locale(tag).unwrap(), &args)
};

assert_eq!(show("en-US", "{$d :datetime}"), "Oct 27, 2026, 3:45\u{202f}PM");
assert_eq!(show("en-US", "{$d :datetime dateStyle=full}"), "Tuesday, October 27, 2026");
assert_eq!(show("en-US", "{$d :datetime timeStyle=long}"), "3:45:30\u{202f}PM");
assert_eq!(
    show("de-DE", "{$d :datetime dateStyle=long timeStyle=short}"),
    "27. Oktober 2026 um 15:45"
);
assert_eq!(
    show("hr", "{$d :datetime dateStyle=full timeStyle=short}"),
    "utorak, 27. listopada 2026. u 15:45"
);
```

A placeholder with a date argument and no function is a `:datetime` with the defaults, so `{$when}` shows the same as the first line.

:::warning Year and month fields cannot take a time
`dateFields` of `year`, `month` or `year-month` cannot be combined with a time of day. `{$d :datetime dateFields=year timeStyle=short}` fails with an "invalid options" `Function` error and shows the fallback `{$d}`. Use `:date fields=year` and a separate `:time`.
:::

## Calendars and the user's preferences

The locale's own calendar is used: `th` shows Buddhist-era years (`2569`). A user preference in the locale's `-u-` extension is honored as well, for example `en-u-ca-buddhist` shows `Oct 27, 2569 BE`, and `-u-hc-h23` sets the clock. The [Translator](./translator.md#user-preferences) passes the user's extensions to every message.

## What goes wrong

| Input | Result |
|---|---|
| A number or no operand | `BadOperand`; the placeholder shows its fallback. |
| Text that is not an ISO 8601 local date or date-time (`2026-13-01`, `...Z`) | `BadOperand`. |
| An option value outside the table (`style=huge`) | `BadOption`; the option is ignored and the date still formats. |
| `:time` on a date without a time of day | Shows midnight (`12:00 AM`). |
| `:date` on a date with a time of day | The time of day is ignored. |

None of the date functions select: a matcher on them can only use `*`.

Next: [Bundles and JSON](./bundles.md) for organizing translations, or [Arguments and values](./arguments.md).
