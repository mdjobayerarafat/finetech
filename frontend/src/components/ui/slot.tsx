import * as React from "react";

const Slot = React.forwardRef<
  HTMLElement,
  React.HTMLAttributes<HTMLElement> & {
    children?: React.ReactNode;
  }
>(({ children, ...props }, ref) => {
  if (!React.isValidElement(children)) {
    return null;
  }
  // children is a valid React element
  return React.cloneElement(children, {
    ...props,
    ...(children.props ?? {}),
    ref: (children as unknown as { ref?: React.Ref<HTMLElement> }).ref
      ? Object.assign(
          ref ?? React.createRef(),
          (children as unknown as { ref: React.Ref<HTMLElement> }).ref
        )
      : ref,
  });
});
Slot.displayName = "Slot";

export { Slot };
