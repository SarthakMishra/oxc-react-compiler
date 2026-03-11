function Display({ items }) {
    const count = items.length;
    const label = count > 0 ? "Has items" : "Empty";
    return <div>{label}: {count}</div>;
}
