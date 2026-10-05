import { useRef, useState } from "react";
import { peekNav, popNav, pushNav } from "../navigation/navStack";

export type MagiTicketFrom = "task-manager" | "week-ahead";

export type MagiTicket = {
  from: MagiTicketFrom;
  taskId: string;
};

export function openMagiCutDraws(args: {
  from: MagiTicketFrom;
  restore: () => void;
  showElements: () => void;
}) {
  pushNav({
    id: "magi-cut-draws",
    screen: args.from === "task-manager" ? "task-manager" : "week-ahead",
    cmDesk: args.from === "week-ahead" ? "weekly" : undefined,
    restore: args.restore,
  });
  args.showElements();
}

type CommandClient = {
  executeCommand: (
    name: string,
    body?: unknown,
  ) => Promise<{ ok: boolean; errorCode?: string }>;
};

export function useMagiCliffNav(
  setScreen: (screen: "task-manager" | "cash-management") => void,
  setCmDesk: (desk: "weekly" | "elements" | "car") => void,
) {
  const [ticket, setTicket] = useState<MagiTicket | null>(null);
  const [cutTaskId, setCutTaskId] = useState<string | null>(null);
  const [drawFilter, setDrawFilter] = useState(false);
  const ticketRef = useRef(ticket);
  const cutTaskIdRef = useRef(cutTaskId);
  const drawFilterRef = useRef(drawFilter);
  const closingRef = useRef(false);
  const dismissRef = useRef(false);
  ticketRef.current = ticket;
  cutTaskIdRef.current = cutTaskId;
  drawFilterRef.current = drawFilter;

  const returnTo = (from: MagiTicketFrom) => {
    if (from === "task-manager") {
      setScreen("task-manager");
    } else {
      setCmDesk("weekly");
      setScreen("cash-management");
    }
  };

  const open = (from: MagiTicketFrom, taskId: string) => {
    setDrawFilter(false);
    setCutTaskId(null);
    setTicket({ from, taskId });
  };

  const close = () => {
    if (peekNav()?.id === "magi-cut-draws") {
      dismissRef.current = true;
      popNav();
      return;
    }
    setTicket(null);
    setCutTaskId(null);
    setDrawFilter(false);
  };

  const cutDraws = () => {
    const ticketNow = ticketRef.current;
    if (!ticketNow || drawFilterRef.current) return;
    openMagiCutDraws({
      from: ticketNow.from,
      restore: () => {
        setDrawFilter(false);
        setCutTaskId(null);
        if (closingRef.current) {
          closingRef.current = false;
          setTicket(null);
          setCmDesk("car");
          setScreen("cash-management");
          return;
        }
        if (dismissRef.current) {
          dismissRef.current = false;
          setTicket(null);
          returnTo(ticketNow.from);
          return;
        }
        setTicket(ticketNow);
        returnTo(ticketNow.from);
      },
      showElements: () => {
        setDrawFilter(true);
        setCutTaskId(ticketNow.taskId);
        setTicket(null);
        setCmDesk("elements");
        setScreen("cash-management");
      },
    });
  };

  const elementAdjusted = async (client: CommandClient) => {
    const taskId = cutTaskIdRef.current;
    if (!drawFilterRef.current || !taskId) return false;
    const result = await client.executeCommand("TaskResolve", {
      taskId,
      how: "done",
    });
    if (!result.ok) return false;
    closingRef.current = true;
    if (peekNav()?.id === "magi-cut-draws") {
      popNav();
    } else {
      closingRef.current = false;
      setDrawFilter(false);
      setCutTaskId(null);
      setTicket(null);
      setCmDesk("car");
      setScreen("cash-management");
    }
    return true;
  };

  return { ticket, cutTaskId, drawFilter, open, close, cutDraws, elementAdjusted };
}
