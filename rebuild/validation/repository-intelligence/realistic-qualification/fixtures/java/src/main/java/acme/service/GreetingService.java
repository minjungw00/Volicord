package acme.service;

import acme.api.GreetingPort;
import acme.util.Names;

public class GreetingService implements GreetingPort {
    @Override
    public String render(String name) {
        return Names.normalize(name);
    }

    public String render(String name, int repeat) {
        return render(name).repeat(repeat);
    }
}
