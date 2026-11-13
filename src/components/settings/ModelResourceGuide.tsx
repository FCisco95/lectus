import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface HardwareProfile {
  logical_cpus: number;
  total_memory_bytes: number | null;
  available_memory_bytes: number | null;
}

export function ModelResourceGuide() {
  const [profile, setProfile] = useState<HardwareProfile | null>(null);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    let active = true;
    invoke<HardwareProfile>('hardware_profile').then(value => { if (active) setProfile(value); })
      .catch(() => { if (active) setFailed(true); });
    return () => { active = false; };
  }, []);
  const gib = (bytes: number) => `${(bytes / 1024 ** 3).toFixed(1)} GB`;
  const lowMemory = profile?.available_memory_bytes != null && profile.available_memory_bytes < 2 * 1024 ** 3;
  return <aside className="model-card" aria-label="Model resource guidance">
    <b>Choose for this computer</b>
    <p className="field-hint">{profile
      ? `${profile.logical_cpus} logical processors${profile.total_memory_bytes != null ? ` · ${gib(profile.total_memory_bytes)} system RAM` : ''}${profile.available_memory_bytes != null ? ` · ${gib(profile.available_memory_bytes)} available when checked` : ''}.`
      : failed ? 'Memory information is unavailable. Start with a smaller model and test it.' : 'Checking system memory…'}</p>
    <p className="field-hint">Start with Base for a short sample in your spoken language. Try Tiny if memory is limited; compare a larger model if recognition misses words. Your accent, noise and graphics hardware affect the result.</p>
    <p className="field-hint">Upstream whisper.cpp estimates: Tiny ~273 MB, Base ~388 MB, Small ~852 MB, Medium ~2.1 GB, Large ~3.9 GB of memory. Lectus, other apps, cleanup and translation need extra memory. These estimates do not measure this computer's speed or GPU memory.</p>
    {lowMemory && <p className="field-hint" role="status">Less than 2 GB of system RAM is available. Close other apps before trying a large model. Local text translation also loads a separate model.</p>}
    <p className="field-hint">Model card sizes below are download sizes. Downloads check free disk space and verify the complete file before it becomes available.</p>
  </aside>;
}
