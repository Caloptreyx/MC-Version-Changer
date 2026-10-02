import { faCube } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Avatar } from '@mantine/core';

export default function TypeIcon({ url, name, size = 48 }: { url: string | null; name: string; size?: number }) {
  return (
    <Avatar src={url || null} alt={name} size={size} radius='md' color='gray' variant='light'>
      <FontAwesomeIcon icon={faCube} size={size >= 48 ? 'lg' : undefined} />
    </Avatar>
  );
}
