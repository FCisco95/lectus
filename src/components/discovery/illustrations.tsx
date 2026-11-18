/** Spotlight artwork: original, abstract 280x120 scenes in the logo palette
 *  (green, orange, blue and coral petals around the ink ring). Colours come
 *  from classes in src/styles/discovery.css so light and dark themes adapt.
 *  The top-right corner (x > 238, y < 40) stays empty for the close button. */

type Petal = 'green' | 'orange' | 'blue' | 'coral';

const PETALS: ReadonlyArray<[Petal, number, number]> = [
  ['green', -40, 40],
  ['orange', 50, 130],
  ['blue', 140, 220],
  ['coral', 230, 310],
];

/** Arc from `from` to `to` degrees, clockwise, 0 pointing up. */
function arc(cx: number, cy: number, r: number, from: number, to: number) {
  const point = (deg: number) => {
    const a = (deg * Math.PI) / 180;
    return `${(cx + r * Math.sin(a)).toFixed(2)} ${(cy - r * Math.cos(a)).toFixed(2)}`;
  };
  return `M${point(from)}A${r} ${r} 0 0 1 ${point(to)}`;
}

/** The Lectus mark reduced to shapes: four petals around the ink ring. */
function Mark({ x, y, size }: { x: number; y: number; size: number }) {
  return (
    <g>
      {PETALS.map(([colour, from, to]) => (
        <path key={colour} d={arc(x, y, size * 0.72, from, to)} className={`da-petal da-stroke-${colour}`}
          strokeWidth={size * 0.42} />
      ))}
      <circle cx={x} cy={y} r={size * 0.44} className="da-ink" />
      <circle cx={x} cy={y} r={size * 0.2} className="da-paper" />
    </g>
  );
}

function Chip({ x, y, w, dot, faded }: { x: number; y: number; w: number; dot?: Petal; faded?: boolean }) {
  const textX = x + (dot ? 21 : 11);
  return (
    <g opacity={faded ? 0.5 : 1}>
      <rect x={x} y={y} width={w} height={20} rx={10} className="da-chip" />
      {dot && <circle cx={x + 11} cy={y + 10} r={3.5} className={`da-fill-${dot}`} />}
      <rect x={textX} y={y + 8} width={x + w - 11 - textX} height={4} rx={2} className="da-text" />
    </g>
  );
}

/** Word chips drifting past one that Lectus has learned. */
export function VocabularyArt() {
  return (
    <svg viewBox="0 0 280 120" className="da-svg" aria-hidden="true" focusable="false">
      <Chip x={20} y={14} w={66} dot="blue" faded />
      <Chip x={94} y={14} w={92} dot="green" />
      <Chip x={194} y={14} w={40} faded />
      <rect x={60} y={45} width={160} height={30} rx={15} className="da-chip da-chip-focus" />
      <Mark x={78} y={60} size={10.5} />
      <rect x={96} y={56} width={74} height={8} rx={4} className="da-text-strong" />
      <path d="M186 60.5l5 5 10-11" className="da-check" />
      <Chip x={26} y={86} w={84} dot="orange" />
      <Chip x={118} y={86} w={58} faded />
      <Chip x={184} y={86} w={78} dot="coral" faded />
    </svg>
  );
}

const BARS: ReadonlyArray<[number, Petal]> = [
  [10, 'green'], [22, 'blue'], [32, 'orange'], [18, 'coral'], [26, 'green'], [14, 'blue'],
];

/** A stack of saved transcripts, the newest with its voice wave. */
export function DictationsArt() {
  return (
    <svg viewBox="0 0 280 120" className="da-svg" aria-hidden="true" focusable="false">
      <rect x={88} y={10} width={124} height={56} rx={12} className="da-chip" opacity={0.45} />
      <rect x={72} y={20} width={156} height={60} rx={13} className="da-chip" opacity={0.75} />
      <rect x={54} y={32} width={190} height={74} rx={14} className="da-chip da-chip-focus" />
      {BARS.map(([h, colour], i) => (
        <rect key={i} x={72 + i * 8.5} y={69 - h / 2} width={4.5} height={h} rx={2.25} className={`da-fill-${colour}`} />
      ))}
      <rect x={138} y={53} width={88} height={6} rx={3} className="da-text-strong" />
      <rect x={138} y={66} width={70} height={6} rx={3} className="da-text" />
      <rect x={138} y={79} width={80} height={6} rx={3} className="da-text" />
    </svg>
  );
}

/** Three model sizes side by side; the middle one is selected. */
export function ModelsArt() {
  const tiles = [
    { x: 41, size: 9, label: 24, selected: false },
    { x: 111, size: 13, label: 32, selected: true },
    { x: 181, size: 17.5, label: 40, selected: false },
  ];
  return (
    <svg viewBox="0 0 280 120" className="da-svg" aria-hidden="true" focusable="false">
      {tiles.map((t) => {
        const cx = t.x + 29;
        return (
          <g key={t.x} opacity={t.selected ? 1 : 0.62}>
            <rect x={t.x} y={22} width={58} height={84} rx={13}
              className={t.selected ? 'da-chip da-chip-focus da-selected' : 'da-chip'} />
            <Mark x={cx} y={56} size={t.size} />
            <rect x={cx - t.label / 2} y={84} width={t.label} height={5} rx={2.5} className={t.selected ? 'da-text-strong' : 'da-text'} />
            <rect x={cx - 9} y={94} width={18} height={4} rx={2} className="da-text" />
          </g>
        );
      })}
      <circle cx={166} cy={24} r={8} className="da-badge" />
      <path d="M162.4 24.2l2.4 2.4 4.6-5" className="da-badge-check" />
    </svg>
  );
}

/** Rough words in, clean words out, with a pair of bubbles for translation. */
export function CleanupArt() {
  return (
    <svg viewBox="0 0 280 120" className="da-svg" aria-hidden="true" focusable="false">
      <rect x={16} y={34} width={88} height={60} rx={13} className="da-chip" opacity={0.8} />
      <path d="M28 52q5-5 10 0t10 0t10 0t10 0t10 0t8 0" className="da-wave" />
      <path d="M28 66q5-5 10 0t10 0t10 0t10 0" className="da-wave" />
      <path d="M28 80q5-5 10 0t10 0t10 0t10 0t10 0" className="da-wave" />
      <circle cx={80} cy={65} r={3.2} className="da-fill-coral" />
      <circle cx={91} cy={79} r={3.2} className="da-fill-coral" opacity={0.7} />
      <path d="M110 64h12" className="da-link" />
      <Mark x={140} y={64} size={13} />
      <path d="M158 64h12" className="da-link" />
      <rect x={176} y={34} width={88} height={60} rx={13} className="da-chip da-chip-focus" />
      <rect x={188} y={48} width={62} height={6} rx={3} className="da-text-strong" />
      <rect x={188} y={61} width={50} height={6} rx={3} className="da-text" />
      <rect x={188} y={74} width={56} height={6} rx={3} className="da-text" />
      <rect x={194} y={86} width={34} height={18} rx={9} className="da-fill-green" />
      <rect x={202} y={93} width={18} height={4} rx={2} className="da-on-colour" />
      <rect x={222} y={93} width={34} height={18} rx={9} className="da-fill-blue da-bubble-front" />
      <rect x={230} y={100} width={18} height={4} rx={2} className="da-on-colour" />
    </svg>
  );
}
