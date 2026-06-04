const SWATCHES = ['#1db584', '#17a2a2', '#3b82f6', '#e91e8c', '#ff6b5a', '#ffa500'];

export function AboutPanel() {
  return (
    <div>
      <h2 className="settings-panel-title">About</h2>
      <p className="settings-panel-sub">Voice to text, everywhere.</p>

      <div className="about-orb" />
      <div style={{ fontSize: 22, fontWeight: 700 }}>
        <span
          style={{
            background:
              'linear-gradient(120deg,#1db584,#17a2a2,#3b82f6,#e91e8c,#ff6b5a,#ffa500)',
            WebkitBackgroundClip: 'text',
            backgroundClip: 'text',
            WebkitTextFillColor: 'transparent',
          }}
        >
          Lectus
        </span>{' '}
        <span style={{ fontSize: 14, fontWeight: 500, color: '#9a9aa2' }}>v0.4.0</span>
      </div>
      <p className="field-hint" style={{ maxWidth: 420, marginTop: 10 }}>
        Hold or tap to talk; Lectus transcribes locally or in the cloud and pastes wherever you
        point. Named after the Eclectus parrot — colourful and a great talker.
      </p>

      <div className="swatches">
        {SWATCHES.map((c) => (
          <span className="swatch" key={c} style={{ background: c }} />
        ))}
      </div>
    </div>
  );
}
