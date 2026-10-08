import { ReturnToPrevious } from "./ReturnToPrevious";
import { SectionNav } from "./SectionNav";

/**
 * The row above the page title: Return first, then this screen's section
 * shortcuts. Both children hide themselves when they have nothing to offer, and
 * `.page-nav-row:empty` then takes the row out of the flow.
 */
export function PageNavRow({
  screen,
  cmDesk,
  onReturn,
}: {
  screen: string;
  cmDesk?: string;
  onReturn: () => void;
}) {
  return (
    <div className="page-nav-row">
      <ReturnToPrevious onReturn={onReturn} />
      <SectionNav screen={screen} cmDesk={cmDesk} />
    </div>
  );
}
