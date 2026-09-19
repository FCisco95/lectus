import logo from '../assets/logo.png';

/** The Lectus mark — used in the titlebar, pill, onboarding, and About. */
export function BrandMark({ className, title }: { className?: string; title?: string }) {
  return <img className={className} src={logo} alt="" title={title} draggable={false} />;
}
