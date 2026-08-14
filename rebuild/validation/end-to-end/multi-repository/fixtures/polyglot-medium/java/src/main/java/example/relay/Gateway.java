package example.relay;

public final class Gateway {
    public String route(Route route) {
        return route.name().trim() + ":" + route.locale();
    }
}
