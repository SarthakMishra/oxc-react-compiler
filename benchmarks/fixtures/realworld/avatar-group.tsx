// XS tier - Inspired by shadcn/ui avatar component
import { useMemo } from 'react';

interface AvatarGroupProps {
  users: { name: string; image?: string }[];
  max?: number;
}

export function AvatarGroup({ users, max = 3 }: AvatarGroupProps) {
  const visible = useMemo(() => users.slice(0, max), [users, max]);
  const remaining = users.length - max;

  return (
    <div className="flex -space-x-2">
      {visible.map((user, i) => (
        <div key={i} className="rounded-full w-8 h-8 bg-gray-300" title={user.name}>
          {user.image ? <img src={user.image} alt={user.name} /> : user.name[0]}
        </div>
      ))}
      {remaining > 0 && <div className="rounded-full w-8 h-8 bg-gray-200">+{remaining}</div>}
    </div>
  );
}
