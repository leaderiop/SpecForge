import { ReactNode } from 'react';
import { motion, AnimatePresence } from 'framer-motion';

interface MorphTransitionProps {
  showAfter: boolean;
  before: ReactNode;
  after: ReactNode;
}

export function MorphTransition({ showAfter, before, after }: MorphTransitionProps) {
  return (
    <AnimatePresence mode="wait">
      {showAfter ? (
        <motion.div
          key="after"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.5 }}
        >
          {after}
        </motion.div>
      ) : (
        <motion.div
          key="before"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.5 }}
        >
          {before}
        </motion.div>
      )}
    </AnimatePresence>
  );
}
