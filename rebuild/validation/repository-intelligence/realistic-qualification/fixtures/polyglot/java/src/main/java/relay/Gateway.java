package relay;

public class Gateway {
    public static final String ENDPOINT = "/v1/greet";
    public static final String SCHEMA = "greeting.request.v1";

    public String enqueue(String name) {
        return SCHEMA + ":" + name.trim();
    }
}
