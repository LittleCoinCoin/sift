<div class="caustics" aria-hidden="true">
  <div class="layer layer-a"></div>
  <div class="layer layer-b"></div>
  <div class="layer layer-c"></div>
</div>

<style>
  .caustics {
    position: fixed;
    inset: 0;
    z-index: var(--z-caustics);
    pointer-events: none;
    overflow: hidden;
  }

  .layer {
    position: absolute;
    inset: -30%;
    border-radius: 50%;
    filter: blur(48px);
    opacity: 0.12;
    mix-blend-mode: screen;
  }

  .layer-a {
    background: radial-gradient(ellipse 60% 40% at 40% 50%, #5BB8D4 0%, transparent 70%),
                radial-gradient(ellipse 35% 55% at 70% 30%, #A8DDE9 0%, transparent 65%);
    animation: drift-a 22s ease-in-out infinite alternate;
  }

  .layer-b {
    background: radial-gradient(ellipse 45% 60% at 60% 60%, #3FA3C1 0%, transparent 70%),
                radial-gradient(ellipse 50% 35% at 25% 70%, #7CCCE3 0%, transparent 60%);
    animation: drift-b 18s ease-in-out infinite alternate;
  }

  .layer-c {
    background: radial-gradient(ellipse 55% 45% at 50% 40%, #B8E6F0 0%, transparent 65%),
                radial-gradient(ellipse 30% 50% at 75% 65%, #5BB8D4 0%, transparent 70%);
    animation: drift-c 25s ease-in-out infinite alternate;
  }

  @keyframes drift-a {
    0%   { transform: scale(1)    translate(0%,   0%)   rotate(0deg); }
    33%  { transform: scale(1.08) translate(4%,  -3%)   rotate(2deg); }
    66%  { transform: scale(0.96) translate(-3%, 5%)    rotate(-1deg); }
    100% { transform: scale(1.05) translate(2%,  3%)    rotate(3deg); }
  }

  @keyframes drift-b {
    0%   { transform: scale(1.05) translate(-2%, 2%)   rotate(-2deg); }
    40%  { transform: scale(0.97) translate(5%,  -4%)  rotate(1deg); }
    70%  { transform: scale(1.10) translate(-4%, -2%)  rotate(-3deg); }
    100% { transform: scale(1)    translate(3%,  4%)   rotate(2deg); }
  }

  @keyframes drift-c {
    0%   { transform: scale(0.98) translate(3%,  -2%)  rotate(1deg); }
    30%  { transform: scale(1.06) translate(-5%, 3%)   rotate(-2deg); }
    60%  { transform: scale(1)    translate(2%,  5%)   rotate(3deg); }
    100% { transform: scale(1.04) translate(-3%, -4%)  rotate(-1deg); }
  }

  /* Tone down in dark mode — pool shimmer stays but dimmer */
  :global([data-theme="dark"]) .layer {
    opacity: 0.07;
    mix-blend-mode: overlay;
  }
</style>
