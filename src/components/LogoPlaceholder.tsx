/**
 * LogoPlaceholder — a clean slot where you can replace the SVG
 * with your own custom logo. The placeholder shows a penguin
 * silhouette as a nod to the project name.
 *
 * To add your logo: replace the SVG below or import an image file.
 */
export default function LogoPlaceholder() {
  return (
    <div className="logo-placeholder">
      <svg
        width="80"
        height="80"
        viewBox="0 0 80 80"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
      >
        {/* Penguin body */}
        <ellipse cx="40" cy="45" rx="22" ry="28" fill="white" stroke="white" strokeWidth="2" />
        {/* Belly */}
        <ellipse cx="40" cy="48" rx="14" ry="18" fill="black" />
        {/* Eyes */}
        <circle cx="33" cy="38" r="4" fill="white" />
        <circle cx="47" cy="38" r="4" fill="white" />
        <circle cx="34" cy="38" r="2" fill="black" />
        <circle cx="48" cy="38" r="2" fill="black" />
        {/* Beak */}
        <polygon points="40,43 37,47 43,47" fill="white" />
        {/* Feet */}
        <ellipse cx="31" cy="72" rx="7" ry="4" fill="white" />
        <ellipse cx="49" cy="72" rx="7" ry="4" fill="white" />
        {/* Flippers */}
        <ellipse cx="17" cy="48" rx="5" ry="14" fill="white" transform="rotate(15, 17, 48)" />
        <ellipse cx="63" cy="48" rx="5" ry="14" fill="white" transform="rotate(-15, 63, 48)" />
      </svg>
      <span className="logo-label">Penguclip</span>
    </div>
  );
}
