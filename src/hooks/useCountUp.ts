import { useEffect, useRef, useState } from 'react';

const useCountUp = (target: number, duration = 800) => {
  const [value, setValue] = useState(0);
  const prev = useRef(0);
  const raf = useRef(0);

  useEffect(() => {
    const start = prev.current;
    const diff = target - start;
    if (diff === 0) return;

    const startTime = performance.now();

    const step = (now: number) => {
      const elapsed = now - startTime;
      const progress = Math.min(elapsed / duration, 1);
      const eased = 1 - Math.pow(1 - progress, 3);
      const current = Math.round(start + diff * eased);

      setValue(current);

      if (progress < 1) {
        raf.current = requestAnimationFrame(step);
      } else {
        prev.current = target;
      }
    };

    raf.current = requestAnimationFrame(step);

    return () => cancelAnimationFrame(raf.current);
  }, [target, duration]);

  return value;
};

export default useCountUp;
