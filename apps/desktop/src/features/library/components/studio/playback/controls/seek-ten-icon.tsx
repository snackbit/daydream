export interface SeekTenIconProps {
  direction: 'back' | 'forward';
}
export function SeekTenIcon({ direction }: SeekTenIconProps) {
  return (
    <svg
      aria-hidden="true"
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
    >
      <g
        transform={
          direction === 'forward' ? 'translate(24 0) scale(-1 1)' : undefined
        }
      >
        <path
          d="M5.5 7.5A8 8 0 1 1 4 12M5.5 3.5v4.5H10"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </g>
      <text
        x="12"
        y="15.5"
        textAnchor="middle"
        fill="currentColor"
        fontSize="8"
        fontWeight="600"
      >
        10
      </text>
    </svg>
  );
}
