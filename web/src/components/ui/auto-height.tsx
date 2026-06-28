import * as React from "react"

/**
 * Wraps children in a container whose `height` tracks the measured content
 * height, allowing CSS height transitions on content-driven layouts.
 *
 * The outer element gets the height transition (pass via `outerClassName`);
 * the inner element holds the actual layout (pass via `innerClassName`).
 * Initial mount sets the height synchronously so opening animations don't
 * stutter; subsequent changes animate via the transition class.
 */
export function AutoHeight({
  children,
  outerClassName,
  innerClassName,
}: {
  children: React.ReactNode
  outerClassName?: string
  innerClassName?: string
}) {
  const outerRef = React.useRef<HTMLDivElement>(null)
  const innerRef = React.useRef<HTMLDivElement>(null)

  React.useLayoutEffect(() => {
    const outer = outerRef.current
    const inner = innerRef.current
    if (!outer || !inner) return
    if (typeof ResizeObserver === "undefined") return

    let raf = 0

    const apply = (animate: boolean) => {
      const target = inner.offsetHeight
      if (target <= 0) return
      if (!animate) {
        outer.style.transition = "none"
        outer.style.height = `${target}px`
        // Force reflow so the non-animated height is committed before
        // re-enabling transitions on the next frame.
        void outer.offsetHeight
        outer.style.transition = ""
        return
      }
      cancelAnimationFrame(raf)
      raf = requestAnimationFrame(() => {
        outer.style.height = `${target}px`
      })
    }

    apply(false)

    const ro = new ResizeObserver(() => apply(true))
    ro.observe(inner)
    return () => {
      cancelAnimationFrame(raf)
      ro.disconnect()
    }
  }, [])

  return (
    <div ref={outerRef} className={outerClassName}>
      <div ref={innerRef} className={innerClassName}>
        {children}
      </div>
    </div>
  )
}
